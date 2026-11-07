"""The website's generate step, from spec.md "Architecture" (website) and issue 12."""

from pathlib import Path

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

    generate_website.generate(source, out, api_host="api.example.test")

    assert (out / "agents" / "index.html").read_text(encoding="utf-8") == (
        '<a href="https://api.example.test/v1/openapi.yaml">API</a>'
    )


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

    generate_website.generate(source, out, api_host="api.example.test")

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

    generate_website.generate(source, out, api_host="api.example.test")

    assert sorted(p.relative_to(out).as_posix() for p in out.rglob("*")) == ["index.html"]


def test_an_unknown_placeholder_in_a_page_fails(tmp_path):
    source = tmp_path / "website"
    source.mkdir()
    (source / "index.html").write_text("{{api_host}} {{domain}}", encoding="utf-8")

    with pytest.raises(generate_website.GenerateError, match=r"index\.html.*\{\{domain\}\}"):
        generate_website.generate(source, tmp_path / "out", api_host="api.example.test")


def test_the_placeholder_pages_link_the_constitution_and_api_description(tmp_path):
    out = tmp_path / "out"
    generate_website.generate(REPO_WEBSITE, out, api_host="api.example.test")

    for page in ("index.html", "agents/index.html"):
        html = (out / page).read_text(encoding="utf-8")
        assert html.startswith("<!DOCTYPE html>"), page
        assert 'href="https://api.example.test/v1/world/constitution"' in html, page
        assert 'href="https://api.example.test/v1/openapi.yaml"' in html, page
