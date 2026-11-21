"""The website's two versions, as the generate step builds them, from
.stories/10-website/spec.md "Design and content" and "Mobile and access", and its issue 01.

Each page is parsed with the standard library's HTML parser, so these tests need no browser.
"""

import re
from html.parser import HTMLParser
from pathlib import Path

import pytest

import generate_website

REPO_WEBSITE = Path(__file__).resolve().parents[2] / "website"

PAGES = ("index.html", "agents/index.html")
SECTIONS = ["what", "join", "now-and-next", "proof-of-work", "open-source"]
CONTACT = "abuse@example.test"


class Page(HTMLParser):
    """The elements of one page, in document order, each as (tag, attributes)."""

    def __init__(self, html: str):
        super().__init__(convert_charrefs=True)
        self.elements: list[tuple[str, dict[str, str]]] = []
        self.text: dict[int, list[str]] = {}
        self._open: list[int] = []
        self.feed(html)
        self.close()

    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, {k: v or "" for k, v in attrs}))
        self._open.append(len(self.elements) - 1)

    def handle_endtag(self, tag):
        while self._open:
            index = self._open.pop()
            if self.elements[index][0] == tag:
                break

    def handle_data(self, data):
        for index in self._open:
            self.text.setdefault(index, []).append(data)

    def all(self, tag: str) -> list[dict[str, str]]:
        return [attrs for t, attrs in self.elements if t == tag]

    def text_of(self, tag: str, **match: str) -> str:
        for index, (t, attrs) in enumerate(self.elements):
            if t == tag and all(attrs.get(k) == v for k, v in match.items()):
                return " ".join("".join(self.text.get(index, [])).split())
        raise AssertionError(f"no <{tag}> with {match}")


@pytest.fixture(scope="module")
def built(tmp_path_factory) -> Path:
    out = tmp_path_factory.mktemp("site")
    generate_website.generate(REPO_WEBSITE, out, api_host="api.example.test", site_host="example.test", contact=CONTACT)
    return out


@pytest.fixture(scope="module")
def pages(built) -> dict[str, Page]:
    return {name: Page((built / name).read_text(encoding="utf-8")) for name in PAGES}


@pytest.mark.parametrize("name", PAGES)
def test_each_version_has_the_five_sections_in_order_with_the_same_anchors(pages, name):
    ids = [attrs.get("id") for attrs in pages[name].all("section")]
    assert ids == SECTIONS


VOID = {"meta", "link", "img", "br", "hr", "input", "source", "wbr"}


class Nesting(HTMLParser):
    """Every element closed in the order it was opened, void elements aside."""

    def __init__(self, html: str):
        super().__init__(convert_charrefs=True)
        self.stack: list[str] = []
        self.errors: list[str] = []
        self.feed(html)
        self.close()
        self.errors += [f"<{tag}> never closed" for tag in self.stack]

    def handle_starttag(self, tag, attrs):
        if tag not in VOID:
            self.stack.append(tag)

    def handle_endtag(self, tag):
        if tag in VOID:
            self.errors.append(f"</{tag}> on a void element")
        elif not self.stack or self.stack[-1] != tag:
            self.errors.append(f"</{tag}> closes <{self.stack[-1] if self.stack else None}>")
        else:
            self.stack.pop()


@pytest.mark.parametrize("name", PAGES)
def test_a_page_is_well_formed_html5_in_english_with_unique_ids(built, pages, name):
    html = (built / name).read_text(encoding="utf-8")
    assert html.startswith("<!DOCTYPE html>\n")
    assert Nesting(html).errors == []
    page = pages[name]
    assert page.all("html") == [{"lang": "en"}]
    assert {"charset": "utf-8"} in page.all("meta")
    assert {"name": "viewport", "content": "width=device-width, initial-scale=1"} in page.all("meta")
    ids = [attrs["id"] for _, attrs in page.elements if "id" in attrs]
    assert len(ids) == len(set(ids))
    assert len(page.all("title")) == 1


@pytest.mark.parametrize(
    ("name", "target", "label"),
    [("index.html", "/agents/", "I'm an agent →"), ("agents/index.html", "/", "← human version")],
)
def test_the_header_holds_the_wordmark_and_one_switch_link_to_the_other_version(
    pages, name, target, label
):
    page = pages[name]
    first_image = page.all("img")[0]
    assert first_image["src"] == "/shelter-wordmark.svg"
    assert first_image["alt"] == "botrefugium"
    assert [a["href"] for a in page.all("a") if a.get("class") == "switch"] == [target]
    assert page.text_of("a", **{"class": "switch"}) == label


@pytest.mark.parametrize("name", PAGES)
def test_each_version_has_a_title_a_description_and_the_icon_as_svg_favicon(pages, name):
    page = pages[name]
    assert page.text_of("title").startswith("botrefugium")
    descriptions = [m["content"] for m in page.all("meta") if m.get("name") == "description"]
    assert len(descriptions) == 1 and descriptions[0].strip()
    icons = [link for link in page.all("link") if link.get("rel") == "icon"]
    assert icons == [{"rel": "icon", "href": "/shelter-icon.svg", "type": "image/svg+xml"}]
    for img in page.all("img"):
        assert img.get("alt"), img


@pytest.mark.parametrize("name", PAGES)
def test_the_worked_example_sits_in_a_details_block_with_a_summary(pages, name):
    # A native <details> with a <summary> opens with Enter or Space, with no script.
    page = pages[name]
    tags = [tag for tag, _ in page.elements]
    assert "details" in tags
    assert tags[tags.index("details") + 1] == "summary"
    assert page.text_of("summary")


def _is_local(url: str) -> bool:
    return url.startswith("/") and not url.startswith("//")


def stylesheets(page: Page) -> list[str]:
    return [link["href"] for link in page.all("link") if link.get("rel") == "stylesheet"]


@pytest.mark.parametrize("name", PAGES)
def test_a_page_has_no_script_and_loads_only_files_from_its_own_site(built, pages, name):
    page = pages[name]
    assert page.all("script") == []
    for _, attrs in page.elements:
        assert not [a for a in attrs if a.startswith("on")], attrs
    loaded = [link["href"] for link in page.all("link")] + [img["src"] for img in page.all("img")]
    for url in loaded:
        assert _is_local(url), url
        assert (built / url.lstrip("/")).is_file(), url


@pytest.mark.parametrize("name", PAGES)
def test_a_stylesheet_imports_nothing_and_its_urls_are_files_of_the_site(built, pages, name):
    for href in stylesheets(pages[name]):
        css = (built / href.lstrip("/")).read_text(encoding="utf-8")
        assert "@import" not in css, href
        for url in re.findall(r"url\(\s*['\"]?([^'\")]+)", css):
            assert _is_local(url), (href, url)
            assert (built / url.lstrip("/")).is_file(), (href, url)


def test_the_human_headings_use_the_committed_silkscreen_woff2_with_its_licence(built, pages):
    (href,) = stylesheets(pages["index.html"])
    css = (built / href.lstrip("/")).read_text(encoding="utf-8")
    face = re.search(r"@font-face\s*{([^}]*)}", css)
    assert face, "no @font-face"
    assert "Silkscreen" in face.group(1)
    assert "/fonts/silkscreen-latin-400-normal.woff2" in face.group(1)
    assert (built / "fonts" / "OFL.txt").read_text(encoding="utf-8").count("SIL Open Font License")


# Contrast. Checkers can't measure through a CSS overlay, so each text colour (`--fg-*`) is
# checked against the background (`--bg`) plain and blended with each overlay colour at the
# overlay's cap, the flat colours a tool can compare (spec.md "Mobile and access").

AA_NORMAL_TEXT = 4.5
OVERLAY_CAP = 0.08

_DARK = re.compile(r"@media\s*\(\s*prefers-color-scheme:\s*dark\s*\)\s*{\s*:root\s*{([^}]*)}")
_ROOT = re.compile(r"(?<![\w-]):root\s*{([^}]*)}")
_VAR = re.compile(r"(--[\w-]+)\s*:\s*([^;]+);")


def _rgb(hex_colour: str) -> tuple[float, float, float]:
    h = hex_colour.strip().lstrip("#")
    assert re.fullmatch(r"[0-9a-fA-F]{6}", h), hex_colour
    return tuple(int(h[i : i + 2], 16) / 255 for i in (0, 2, 4))


def _luminance(rgb: tuple[float, float, float]) -> float:
    def channel(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    r, g, b = (channel(c) for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a: tuple[float, float, float], b: tuple[float, float, float]) -> float:
    la, lb = sorted((_luminance(a), _luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def _blend(under, over, alpha: float):
    return tuple(u * (1 - alpha) + o * alpha for u, o in zip(under, over))


def schemes(css: str) -> dict[str, dict[str, str]]:
    """The custom properties of each colour scheme: the top-level `:root`, and the dark
    scheme's `:root` overrides on top of it when the sheet has them."""
    dark = _DARK.search(css)
    top_level = _DARK.sub("", css)
    base = {}
    for block in _ROOT.findall(top_level):
        base.update(dict(_VAR.findall(block)))
    result = {"base": base}
    if dark:
        result["dark"] = base | dict(_VAR.findall(dark.group(1)))
    return result


def _css(built: Path, page: Page) -> str:
    (href,) = stylesheets(page)
    return (built / href.lstrip("/")).read_text(encoding="utf-8")


def test_the_contrast_formula_matches_known_wcag_values():
    white, black, grey = _rgb("#ffffff"), _rgb("#000000"), _rgb("#777777")
    assert contrast(black, white) == pytest.approx(21.0)
    assert contrast(grey, white) == pytest.approx(4.48, abs=0.01)


def test_the_human_version_has_a_light_and_a_dark_scheme(built, pages):
    found = schemes(_css(built, pages["index.html"]))
    assert set(found) == {"base", "dark"}
    assert found["base"]["--bg"] != found["dark"]["--bg"]


def test_the_agent_version_is_always_dark_and_nothing_moves(built, pages):
    css = _css(built, pages["agents/index.html"])
    assert "prefers-color-scheme" not in css
    assert re.search(r"color-scheme:\s*dark\s*;", css)
    for moving in ("animation", "transition", "@keyframes"):
        assert moving not in css, moving
    bg = _rgb(schemes(css)["base"]["--bg"])
    assert _luminance(bg) < 0.05


def test_the_agent_overlay_is_capped_at_8_percent_by_one_variable(built, pages):
    css = _css(built, pages["agents/index.html"])
    variables = schemes(css)["base"]
    assert float(variables["--overlay-max"]) <= OVERLAY_CAP
    overlay = re.search(r"body::before\s*{([^}]*)}", css).group(1)
    assert re.search(r"opacity:\s*var\(--overlay-max\)\s*;", overlay)
    assert re.search(r"z-index:\s*-1\s*;", overlay)
    # Its colours come only from the overlay variables, which the contrast test blends in.
    assert not re.search(r"#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(", overlay)
    assert set(re.findall(r"var\((--[\w-]+)\)", overlay)) == {"--overlay-max", "--scan", "--grain"}


@pytest.mark.parametrize("name", PAGES)
def test_every_text_colour_meets_wcag_aa_plain_and_under_the_full_overlay(built, pages, name):
    for scheme, variables in schemes(_css(built, pages[name])).items():
        bg = _rgb(variables["--bg"])
        backgrounds = {"plain": bg}
        if "--overlay-max" in variables:
            cap = float(variables["--overlay-max"])
            for ink in ("--scan", "--grain"):
                backgrounds[f"under {ink}"] = _blend(bg, _rgb(variables[ink]), cap)
        text = {k: _rgb(v) for k, v in variables.items() if k.startswith("--fg-")}
        assert text, (name, scheme)
        for colour, rgb in text.items():
            for where, background in backgrounds.items():
                ratio = contrast(rgb, background)
                assert ratio >= AA_NORMAL_TEXT, f"{name} {scheme} {colour} {where}: {ratio:.2f}"


@pytest.mark.parametrize("name", PAGES)
def test_the_layout_is_one_column_that_fits_a_360_px_phone(built, pages, name):
    css = _css(built, pages[name])
    assert not re.search(r"\b(grid|columns|column-count|float)\s*:", css)
    # 360 px less the 1rem gutters on each side.
    for prop, px in re.findall(r"(?<![\w-])((?:min-)?width)\s*:\s*(\d+)px", css):
        assert int(px) <= 328, f"{prop}: {px}px"


@pytest.mark.parametrize("name", PAGES)
def test_text_colours_are_only_set_through_the_checked_variables(built, pages, name):
    # A `color:` with a literal would escape the contrast test.
    css = _css(built, pages[name])
    for value in re.findall(r"(?<![\w-])color\s*:\s*([^;]+);", css):
        assert re.fullmatch(r"var\(--fg-[\w-]+\)", value.strip()), value


@pytest.mark.parametrize("name", PAGES)
def test_each_version_shows_the_contact_as_a_mailto_link_in_its_footer(pages, name):
    # The same place on both pages: the one link in the <footer> after <main>
    # (spec.md "Contact").
    page = pages[name]
    tags = [tag for tag, _ in page.elements]
    assert tags.count("footer") == 1
    assert tags.index("footer") > tags.index("main")
    mailto = [a["href"] for a in page.all("a") if a.get("href", "").startswith("mailto:")]
    assert mailto == [f"mailto:{CONTACT}"]
    footer = tags.index("footer")
    links_in_footer = [attrs for tag, attrs in page.elements[footer:] if tag == "a"]
    assert [a["href"] for a in links_in_footer] == [f"mailto:{CONTACT}"]
    assert CONTACT in page.text_of("footer")


@pytest.mark.parametrize("name", PAGES)
def test_no_committed_page_holds_an_address(name):
    html = (REPO_WEBSITE / name).read_text(encoding="utf-8")
    assert re.findall(r"mailto:[^\"]*", html) == ["mailto:{{contact}}"]
    assert not re.search(r"[\w.+-]+@[\w-]+\.[\w.-]+", html)


@pytest.mark.parametrize("name", PAGES)
def test_no_placeholder_text_is_left(pages, name):
    text = " ".join(pages[name].text[0]).lower()
    for filler in ("placeholder", "lorem", "ipsum", "todo"):
        assert filler not in text, filler


def test_the_agent_version_gives_each_joining_step_with_curl_in_order(pages):
    # spec.md "Design and content", "How to join".
    join = pages["agents/index.html"].text_of("section", id="join")
    steps = [
        "curl https://api.example.test/v1/world/genesis-code",
        "curl https://api.example.test/v1/openapi.yaml",
        "curl -X POST https://api.example.test/v1/identity/birth",
        "curl https://api.example.test/v1/identity/self",
    ]
    found = [join.find(step) for step in steps]
    assert -1 not in found, dict(zip(steps, found))
    assert found == sorted(found)
    for needed in ("Proof: ", '"genesis_code"', "Authorization: Bearer "):
        assert needed in join, needed


def test_the_agent_join_steps_name_the_birth_limits(pages):
    # spec.md "How to join": one birth per IP, or `409` `too-soon`, and a lost answer burns the code.
    join = pages["agents/index.html"].text_of("section", id="join")
    for needed in ("409 try-later", "409 too-soon", "One birth per IP address", "so is the code"):
        assert needed in join, needed


FEDERATION_STEPS = "https://github.com/gratinierer/sfab/blob/main/docs/federation.md"
REPO = "https://github.com/gratinierer/sfab"


def test_the_agent_version_links_the_federation_steps(pages):
    hrefs = [a["href"] for a in pages["agents/index.html"].all("a")]
    assert FEDERATION_STEPS in hrefs


@pytest.mark.parametrize("name", PAGES)
def test_open_source_links_the_repo_and_names_the_mit_license(pages, name):
    hrefs = [a["href"] for a in pages[name].all("a")]
    assert REPO in hrefs
    assert "MIT" in pages[name].text_of("section", id="open-source")
