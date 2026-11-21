"""The website's generate step, from spec.md "Architecture" (website) and issue 12."""

import json
from pathlib import Path
from xml.etree import ElementTree

import pytest

import generate_website

REPO_WEBSITE = Path(__file__).resolve().parents[2] / "website"


def test_a_page_gets_the_api_host_in_place_of_its_placeholder(tmp_path):
    source = tmp_path / "website"
    (source / "agents").mkdir(parents=True)
    (source / "agents" / "index.html").write_text(
        '<a href="https://{{api_host}}/v1/openapi.yaml">API</a>', encoding="utf-8"
    )
    out = tmp_path / "out"

    generate_website.generate(source, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    assert (out / "agents" / "index.html").read_text(encoding="utf-8") == (
        '<a href="https://api.example.test/v1/openapi.yaml">API</a>'
    )


def test_a_page_gets_the_contact_in_place_of_its_placeholder(tmp_path):
    source = tmp_path / "website"
    source.mkdir()
    (source / "index.html").write_text(
        '<a href="mailto:{{contact}}">{{contact}}</a>', encoding="utf-8"
    )
    out = tmp_path / "out"

    generate_website.generate(source, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    assert (out / "index.html").read_text(encoding="utf-8") == (
        '<a href="mailto:abuse@example.test">abuse@example.test</a>'
    )


@pytest.mark.parametrize(
    "contact", ["https://example.test/abuse", "no-at-sign", 'a"b@example.test', "a b@example.test"]
)
def test_a_contact_that_is_not_a_plain_address_fails(tmp_path, contact):
    # It goes into a mailto: link on both pages, so anything else would break the link.
    source = tmp_path / "website"
    source.mkdir()
    (source / "index.html").write_text("{{contact}}", encoding="utf-8")

    with pytest.raises(generate_website.GenerateError, match="abuse_contact"):
        generate_website.generate(source, tmp_path / "out", api_host="api.example.test", site_host="example.test", contact=contact)


def test_every_other_file_is_copied_unchanged(tmp_path):
    source = tmp_path / "website"
    (source / ".well-known" / "sfab-federation").mkdir(parents=True)
    token = source / ".well-known" / "sfab-federation" / "abc"
    token.write_bytes(b"{{api_host}}\n")
    image = source / "icon.svg"
    image.write_bytes(b"<svg>{{api_host}}</svg>")
    style = source / "style.css"
    style.write_bytes(b"body { margin: 0 }\n")
    out = tmp_path / "out"

    generate_website.generate(source, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    for original in (token, image, style):
        assert (out / original.relative_to(source)).read_bytes() == original.read_bytes()


def test_a_rerun_removes_a_file_deleted_from_the_source(tmp_path):
    # BucketDeployment prunes the bucket to the build folder, so a stale file there would
    # stay published.
    source = tmp_path / "website"
    source.mkdir()
    (source / "index.html").write_text("home", encoding="utf-8")
    out = tmp_path / "out"
    (out / "old").mkdir(parents=True)
    (out / "old" / "gone.html").write_text("stale", encoding="utf-8")

    generate_website.generate(source, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    assert sorted(p.relative_to(out).as_posix() for p in out.rglob("*")) == ["index.html"]


def test_an_unknown_placeholder_in_a_page_fails(tmp_path):
    source = tmp_path / "website"
    source.mkdir()
    (source / "index.html").write_text("{{api_host}} {{domain}}", encoding="utf-8")

    with pytest.raises(generate_website.GenerateError, match=r"index\.html.*\{\{domain\}\}"):
        generate_website.generate(source, tmp_path / "out", api_host="api.example.test", site_host="example.test", contact="abuse@example.test")


def test_the_placeholder_pages_link_the_constitution_and_api_description(tmp_path):
    out = tmp_path / "out"
    generate_website.generate(REPO_WEBSITE, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    for page in ("index.html", "agents/index.html"):
        html = (out / page).read_text(encoding="utf-8")
        assert html.startswith("<!DOCTYPE html>"), page
        assert 'href="https://api.example.test/v1/world/constitution"' in html, page
        assert 'href="https://api.example.test/v1/openapi.yaml"' in html, page


@pytest.mark.parametrize("name", ["llms.txt", "sitemap.xml", ".well-known/api-catalog", "index.html"])
def test_a_text_file_gets_the_site_host_in_place_of_its_placeholder(tmp_path, name):
    source = tmp_path / "website"
    (source / name).parent.mkdir(parents=True, exist_ok=True)
    (source / name).write_text("https://{{site_host}}/ https://{{api_host}}/", encoding="utf-8")
    out = tmp_path / "out"

    generate_website.generate(source, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    assert (out / name).read_text(encoding="utf-8") == "https://example.test/ https://api.example.test/"


@pytest.mark.parametrize("name", ["robots.txt", "sitemap.xml", ".well-known/api-catalog"])
def test_an_unknown_placeholder_in_any_text_file_fails(tmp_path, name):
    source = tmp_path / "website"
    (source / name).parent.mkdir(parents=True, exist_ok=True)
    (source / name).write_text("{{domain}}", encoding="utf-8")

    with pytest.raises(generate_website.GenerateError, match=r"\{\{domain\}\}"):
        generate_website.generate(source, tmp_path / "out", api_host="api.example.test", site_host="example.test", contact="abuse@example.test")


def test_the_built_site_leaves_no_placeholder_and_copies_every_other_file_unchanged(tmp_path):
    out = tmp_path / "out"
    generate_website.generate(REPO_WEBSITE, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    text = [p for p in out.rglob("*") if p.is_file() and (p.suffix in (".html", ".txt", ".xml") or p.name == "api-catalog")]
    names = {p.relative_to(out).as_posix() for p in text}
    assert {"index.html", "agents/index.html", "llms.txt", "robots.txt", "sitemap.xml", ".well-known/api-catalog"} <= names
    for path in text:
        assert "{{" not in path.read_text(encoding="utf-8"), path
    for path in out.rglob("*"):
        if path.is_file() and path not in text:
            assert path.read_bytes() == (REPO_WEBSITE / path.relative_to(out)).read_bytes(), path


def test_the_discovery_files_point_to_the_api_description_with_absolute_urls(tmp_path):
    # spec.md "Ways for agents to find the API".
    out = tmp_path / "out"
    generate_website.generate(REPO_WEBSITE, out, api_host="api.example.test", site_host="example.test", contact="abuse@example.test")

    catalog = json.loads((out / ".well-known" / "api-catalog").read_text(encoding="utf-8"))
    (own, api) = catalog["linkset"]
    assert own["anchor"] == "https://example.test/.well-known/api-catalog"
    assert [i["href"] for i in own["item"]] == ["https://api.example.test/v1/openapi.yaml"]
    assert [d["href"] for d in api["service-desc"]] == ["https://api.example.test/v1/openapi.yaml"]

    sitemap = ElementTree.parse(out / "sitemap.xml").getroot()
    locs = [e.text for e in sitemap.iter("{http://www.sitemaps.org/schemas/sitemap/0.9}loc")]
    assert locs == ["https://example.test/", "https://example.test/agents/"]

    robots = (out / "robots.txt").read_text(encoding="utf-8").splitlines()
    assert robots[:2] == ["User-agent: *", "Allow: /"]
    assert "Sitemap: https://example.test/sitemap.xml" in robots

    llms = (out / "llms.txt").read_text(encoding="utf-8")
    for url in (
        "https://api.example.test/v1/openapi.yaml",
        "https://api.example.test/v1/world/constitution",
        "https://example.test/agents/",
        "https://github.com/gratinierer/sfab",
        "https://github.com/gratinierer/sfab/blob/main/CHANGELOG.md",
    ):
        assert f"]({url})" in llms, url
    assert "Proof of work" in llms
