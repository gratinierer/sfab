"""The website's pages, as the generate step builds them: the noir page at `/`, the board page
at `/board/`, the space page at `/space/` and the agent version at `/agents/`, from
.stories/10-website/spec.md "Design and content", "Mobile and access" and "Variants", which
wins where they disagree, and its issues 01, 04, 05, 06 and 07.

Each page is parsed with the standard library's HTML parser, so these tests need no browser.
"""

import importlib.util
import re
from html.parser import HTMLParser
from pathlib import Path

import pytest

import generate_public_openapi
import generate_website


def _load_noircounts():
    """The art generators' counting module, loaded from its file so `scripts/website` never
    goes on `sys.path`; it has no side effects, so the parity tests below can run the board's
    copy of the counting rule against the page tests' own."""
    path = Path(__file__).resolve().parents[2] / "scripts" / "website" / "noircounts.py"
    spec = importlib.util.spec_from_file_location("noircounts", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


noircounts = _load_noircounts()

REPO_WEBSITE = Path(__file__).resolve().parents[2] / "website"

PAGES = ("index.html", "board/index.html", "space/index.html", "agents/index.html")
NOIR, BOARD, SPACE, AGENT = PAGES
# spec.md "Variants" (Sections): the human-side pages use noir's seven, the agent version
# keeps its five, each in its own order.
HUMAN_SECTIONS = [
    "world", "join", "identity", "friends", "homes", "academy", "constitution", "proof-of-work", "open-source", "next",
]
AGENT_SECTIONS = ["what", "join", "now-and-next", "proof-of-work", "open-source"]
SECTIONS = {NOIR: HUMAN_SECTIONS, BOARD: HUMAN_SECTIONS, SPACE: HUMAN_SECTIONS, AGENT: AGENT_SECTIONS}
CONTACT = "abuse@example.test"


class Page(HTMLParser):
    """The elements of one page, in document order, each as (tag, attributes)."""

    def __init__(self, html: str):
        super().__init__(convert_charrefs=True)
        self.elements: list[tuple[str, dict[str, str]]] = []
        self.text: dict[int, list[str]] = {}
        self._open: list[int] = []
        self.ends: dict[int, int] = {}
        self.feed(html)
        self.close()

    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, {k: v or "" for k, v in attrs}))
        self._open.append(len(self.elements) - 1)

    def handle_endtag(self, tag):
        while self._open:
            index = self._open.pop()
            self.ends[index] = len(self.elements)
            if self.elements[index][0] == tag:
                break

    def handle_data(self, data):
        for index in self._open:
            self.text.setdefault(index, []).append(data)

    def all(self, tag: str) -> list[dict[str, str]]:
        return [attrs for t, attrs in self.elements if t == tag]

    def find(self, tag: str | None = None, **attrs: str) -> int:
        """The index of the one element with this tag, if given, and these attribute values,
        failing on none or several; a keyword's `_` stands for `-`, and `class_` for `class`."""
        wanted = {k.rstrip("_").replace("_", "-"): v for k, v in attrs.items()}
        found = [
            i for i, (t, a) in enumerate(self.elements)
            if (tag is None or t == tag) and all(a.get(k) == v for k, v in wanted.items())
        ]
        assert len(found) == 1, f"{len(found)} elements match <{tag or '*'}> {wanted}"
        return found[0]

    def name_of(self, index: int) -> str:
        """The accessible name of the element at `index`: its text and its images' `alt`."""
        images = [attrs.get("alt", "") for _, tag, attrs in self.inside(index) if tag == "img"]
        return " ".join(("".join(self.text.get(index, [])) + " ".join(images)).split())

    def inside(self, index: int) -> list[tuple[int, str, dict[str, str]]]:
        """The elements nested in the element at `index`, each with its own index."""
        end = self.ends.get(index, len(self.elements))
        return [(i, *self.elements[i]) for i in range(index + 1, end)]

    def section_ids(self) -> list[str]:
        """The section ids of either group, in document order, whatever element carries them:
        noir's `<details class="panel">`, the agent version's `<section>`."""
        known = set(HUMAN_SECTIONS) | set(AGENT_SECTIONS)
        return [attrs["id"] for _, attrs in self.elements if attrs.get("id") in known]

    def section(self, section_id: str) -> str:
        """The text of the section with this id, whatever element carries it."""
        for index, (_, attrs) in enumerate(self.elements):
            if attrs.get("id") == section_id:
                return " ".join("".join(self.text.get(index, [])).split())
        raise AssertionError(f"no section #{section_id}")

    def _index_of(self, element_id: str) -> int:
        for index, (_, attrs) in enumerate(self.elements):
            if attrs.get("id") == element_id:
                return index
        raise AssertionError(f"no element #{element_id}")

    def _text(self, index: int) -> str:
        return " ".join("".join(self.text.get(index, [])).split())

    def paragraphs_and_lists(self, section_id: str, noir: bool = False) -> list[tuple[str, str, list[str]]]:
        """The section's paragraphs and lists, each as (tag, text, link targets), leaving out
        its headings. On noir, the source, it also leaves out the themed captions and the
        nested `<details>` (the worked example), which spec.md "Variants" lets noir hold alone;
        on any other page they count, so an extra one fails the comparison."""
        blocks, skip_to = [], -1
        for i, tag, attrs in self.inside(self._index_of(section_id)):
            if i < skip_to:
                continue
            if noir and (tag == "details" or (tag == "p" and attrs.get("class") == "caption")):
                skip_to = self.ends[i]
            elif tag in ("p", "ul", "ol"):
                skip_to = self.ends[i]
                links = [a["href"] for _, t, a in self.inside(i) if t == "a"]
                blocks.append((tag, self._text(i), links))
        return blocks

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
    assert pages[name].section_ids() == SECTIONS[name]


@pytest.mark.parametrize("name", [BOARD, SPACE])
@pytest.mark.parametrize("section_id", HUMAN_SECTIONS)
def test_board_and_space_carry_noirs_paragraphs_and_lists_word_for_word(pages, name, section_id):
    # spec.md "Variants" (Same content): noir's text is the source; only headings may differ.
    noir = pages[NOIR].paragraphs_and_lists(section_id, noir=True)
    assert noir
    assert pages[name].paragraphs_and_lists(section_id) == noir


@pytest.mark.parametrize("name", [BOARD, SPACE])
def test_board_and_space_link_the_worked_example_on_noir(pages, name):
    links = [href for _, _, hrefs in pages[name].paragraphs_and_lists("proof-of-work") for href in hrefs]
    assert "/#proof-of-work" in links


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


# spec.md "Variants" (Four pages, one switch): the same four links, in this order, on every page.
SWITCH = [("/", "I'm human"), ("/board/", "I'm nerd"), ("/space/", "I'm 11"), ("/agents/", "I'm agent")]
HOME = {NOIR: "/", BOARD: "/board/", SPACE: "/space/", AGENT: "/agents/"}


@pytest.mark.parametrize("name", PAGES)
def test_the_header_holds_a_link_home_and_the_four_way_switch_marking_the_current_page(pages, name):
    # spec.md "Variants": a link home to the page named botrefugium, the wordmark image or
    # themed text, and the switch of four links, the current one marked aria-current="page".
    page = pages[name]
    inside = page.inside(page.find("header"))
    (switch,) = [i for i, tag, attrs in inside if tag == "nav" and attrs.get("class") == "switch"]
    in_switch = {i for i, _, _ in page.inside(switch)}

    links = [(i, attrs) for i, tag, attrs in inside if tag == "a"]
    assert [(attrs["href"], page.name_of(i)) for i, attrs in links if i not in in_switch] == [(HOME[name], "botrefugium")]
    assert [(attrs["href"], page.name_of(i)) for i, attrs in links if i in in_switch] == SWITCH
    current = [attrs["href"] for _, attrs in page.elements if "aria-current" in attrs]
    assert current == [HOME[name]]
    assert [attrs["aria-current"] for _, attrs in page.elements if "aria-current" in attrs] == ["page"]


@pytest.mark.parametrize("name", PAGES)
def test_each_version_has_a_title_a_description_and_the_icon_as_svg_favicon(pages, name):
    page = pages[name]
    assert page.text_of("title") == "botrefugium"
    descriptions = [m["content"] for m in page.all("meta") if m.get("name") == "description"]
    assert len(descriptions) == 1 and descriptions[0].strip()
    icons = [link for link in page.all("link") if link.get("rel") == "icon"]
    assert icons == [{"rel": "icon", "href": "/shelter-icon.svg", "type": "image/svg+xml"}]
    for img in page.all("img"):
        assert img.get("alt"), img


@pytest.mark.parametrize("name", [NOIR, AGENT])
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


# spec.md "Variants" (JavaScript): board and space may load scripts from their own folder
# only; noir and the agent version have none.
SCRIPT_FOLDER = {BOARD: "/board/", SPACE: "/space/"}


def scripts(page: Page) -> list[str]:
    return [script.get("src", "") for script in page.all("script")]


@pytest.mark.parametrize("name", PAGES)
def test_a_page_has_scripts_only_from_its_own_folder_and_loads_only_files_from_its_own_site(
    built, pages, name
):
    page = pages[name]
    folder = SCRIPT_FOLDER.get(name)
    if folder is None:
        assert page.all("script") == []
    else:
        assert scripts(page)
        for src in scripts(page):
            assert src.startswith(folder) and (built / src.lstrip("/")).is_file(), src
    for _, attrs in page.elements:
        assert not [a for a in attrs if a.startswith("on")], attrs
    loaded = [link["href"] for link in page.all("link")] + [img["src"] for img in page.all("img")]
    for url in loaded:
        assert _is_local(url), url
        assert (built / url.lstrip("/")).is_file(), url


# What board and space may load from the site root; everything else each uses lives in its own
# folder (issues 05 and 06 "Scope"). Space draws its name in a system font, so it needs neither
# the wordmark nor Silkscreen.
SHARED = {
    BOARD: {"/shelter-icon.svg", "/shelter-wordmark.svg", "/fonts/silkscreen-latin-400-normal.woff2"},
    SPACE: {"/shelter-icon.svg"},
}


@pytest.mark.parametrize("name", list(SHARED))
def test_board_and_space_load_nothing_from_the_site_root_but_their_shared_files(built, pages, name):
    page = pages[name]
    loaded = [link["href"] for link in page.all("link")] + [img["src"] for img in page.all("img")]
    loaded += scripts(page)
    for href in stylesheets(page):
        css = (built / href.lstrip("/")).read_text(encoding="utf-8")
        loaded += re.findall(r"url\(\s*['\"]?([^'\")]+)", css)
    for _, attrs in page.elements:
        loaded += re.findall(r"url\(\s*['\"]?([^'\")]+)", attrs.get("style", ""))
    assert loaded
    folder = SCRIPT_FOLDER[name]
    for url in loaded:
        assert url.startswith(folder) or url in SHARED[name], url
        assert (built / url.lstrip("/")).is_file(), url


@pytest.mark.parametrize("name", PAGES)
def test_a_stylesheet_imports_nothing_and_its_urls_are_files_of_the_site(built, pages, name):
    for href in stylesheets(pages[name]):
        css = (built / href.lstrip("/")).read_text(encoding="utf-8")
        assert "@import" not in css, href
        for url in re.findall(r"url\(\s*['\"]?([^'\")]+)", css):
            assert _is_local(url), (href, url)
            assert (built / url.lstrip("/")).is_file(), (href, url)


def test_space_uses_a_rounded_display_font_from_the_readers_system(built, pages):
    # spec.md "Variants" (Fonts): nothing downloaded.
    css = _css(built, pages[SPACE])
    assert "@font-face" not in css
    display = variables(css)["--display"]
    assert "Rounded" in display and display.rstrip().endswith("sans-serif")


@pytest.mark.parametrize("name", [NOIR, BOARD])
def test_the_human_headings_use_the_committed_silkscreen_woff2_with_its_licence(built, pages, name):
    (href,) = stylesheets(pages[name])
    css = (built / href.lstrip("/")).read_text(encoding="utf-8")
    face = re.search(r"@font-face\s*{([^}]*)}", css)
    assert face, "no @font-face"
    assert "Silkscreen" in face.group(1)
    assert "/fonts/silkscreen-latin-400-normal.woff2" in face.group(1)
    assert (built / "fonts" / "OFL.txt").read_text(encoding="utf-8").count("SIL Open Font License")


# Contrast. Each page has one fixed scheme, its colours in variables, and this table says which
# text variable sits on which background variable on that page (spec.md "Variants" (Colour
# schemes)). Checkers can't measure through a CSS overlay, so where a page has one (the agent
# version's scan lines and grain), each pair is also checked blended with each overlay colour
# at the overlay's cap, the flat colours a tool can compare (spec.md "Mobile and access").

AA_NORMAL_TEXT = 4.5
OVERLAY_CAP = 0.08

_NOIR_GROUNDS = ["--night", "--panel", "--edge"]
_SPACE_GROUNDS = ["--card", "--space"]
TEXT_ON = {
    "index.html": {
        "--ink": _NOIR_GROUNDS,
        "--muted": _NOIR_GROUNDS,
        "--link": _NOIR_GROUNDS,
        "--signal": _NOIR_GROUNDS,
        # The panels' pixel icons take their colour from `color:`.
        "--hero": ["--panel"],
        # The comic captions.
        "--night": ["--signal"],
    },
    BOARD: {
        "--fg-text": ["--bg"],
        "--fg-heading": ["--bg"],
        "--fg-link": ["--bg"],
    },
    SPACE: {
        "--ink": _SPACE_GROUNDS,
        "--muted": _SPACE_GROUNDS,
        # Links.
        "--cyan": _SPACE_GROUNDS,
        # The cards' headings, each in its stop's ink (`--c`), and the footer's title.
        "--pink": ["--card"],
        "--lime": _SPACE_GROUNDS,
        "--orange": ["--card"],
        # The kicker above the title.
        "--space": ["--lime"],
    },
    "agents/index.html": {
        "--fg-text": ["--bg"],
        "--fg-muted": ["--bg"],
        "--fg-cyan": ["--bg"],
        "--fg-rust": ["--bg"],
    },
}

# The site's own SVGs a page's stylesheet sets behind its text: board's text sits on its bots
# with no solid backing, so each text colour is also checked against every fill in them.
SVGS_UNDER_TEXT = {BOARD: {"/board/bot.svg", "/board/bot-faint.svg", "/board/bot-tile.svg"}}

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


def variables(css: str) -> dict[str, str]:
    """The custom properties of the page's one scheme, from its `:root` blocks."""
    found = {}
    for block in _ROOT.findall(css):
        found.update(dict(_VAR.findall(block)))
    return found


def _css(built: Path, page: Page) -> str:
    (href,) = stylesheets(page)
    return (built / href.lstrip("/")).read_text(encoding="utf-8")


def _media_blocks(css: str, query: str) -> tuple[list[str], str]:
    """The bodies of every `@media <query> { ... }`, by matching braces, and the sheet
    without them."""
    inside, outside, at = [], [], 0
    pattern = re.compile(r"@media\s*\(\s*" + re.escape(query) + r"\s*\)\s*{")
    for match in pattern.finditer(css):
        depth, i = 1, match.end()
        while depth:
            depth += {"{": 1, "}": -1}.get(css[i], 0)
            i += 1
        outside.append(css[at : match.start()])
        inside.append(css[match.end() : i - 1])
        at = i
    outside.append(css[at:])
    return inside, "".join(outside)


def test_the_contrast_formula_matches_known_wcag_values():
    white, black, grey = _rgb("#ffffff"), _rgb("#000000"), _rgb("#777777")
    assert contrast(black, white) == pytest.approx(21.0)
    assert contrast(grey, white) == pytest.approx(4.48, abs=0.01)


SCHEME = {NOIR: "dark", BOARD: "light", SPACE: "dark", AGENT: "dark"}
# Bright accent fills that small dark text sits on, such as noir's captions on `--signal` and
# space's kicker on `--lime`; they are grounds in the contrast table but not the page's background, so the dark-scheme
# check leaves them out.
ACCENT_GROUNDS = {NOIR: {"--signal"}, SPACE: {"--lime"}}


@pytest.mark.parametrize("name", PAGES)
def test_each_page_has_one_fixed_scheme(built, pages, name):
    # spec.md "Variants" (Colour schemes): noir dark, board light lilac, space dark, agent
    # dark, none following the browser.
    css = _css(built, pages[name])
    assert "prefers-color-scheme" not in css
    assert re.search(r"color-scheme:\s*" + SCHEME[name] + r"\s*;", css)
    grounds = set().union(*TEXT_ON[name].values()) - ACCENT_GROUNDS.get(name, set())
    for ground in sorted(grounds):
        luminance = _luminance(_rgb(variables(css)[ground]))
        if SCHEME[name] == "light":
            assert luminance > 0.7, ground
        else:
            assert luminance < 0.05, ground


_FILL = re.compile(r'fill\s*(?:=\s*["\']|:\s*)([^"\';>\s]+)')
_HEX = re.compile(r"#[0-9a-fA-F]{6}")
_GRADIENT = re.compile(r"<(?:linear|radial)Gradient\b")
_NO_COLOUR = {"none"}


def _unreadable_fills(svg: str) -> set[str]:
    """The fills `_fills` can't turn into one colour: a name, `rgb()`, `url()`, `currentColor`
    (whose colour the SVG file doesn't hold), a gradient."""
    found = {f for f in _FILL.findall(svg) if f not in _NO_COLOUR and not _HEX.fullmatch(f)}
    return found | ({"a gradient"} if _GRADIENT.search(svg) else set())


def _fills(svg: str) -> set[str]:
    """Every fill colour in the SVG, failing on one it can't read, so no colour escapes the
    contrast check."""
    assert not _unreadable_fills(svg), _unreadable_fills(svg)
    return {f for f in _FILL.findall(svg) if _HEX.fullmatch(f)}


def _css_urls(css: str) -> list[str]:
    return re.findall(r"url\(\s*['\"]?([^'\")]+)", css)


@pytest.mark.parametrize("name", list(SVGS_UNDER_TEXT))
def test_the_svgs_behind_text_are_the_ones_the_contrast_test_knows(built, pages, name):
    # Noir's tiles sit behind solid panels and are decorative; board has no solid panels, so
    # every SVG its stylesheet draws is behind text.
    urls = {url for url in _css_urls(_css(built, pages[name])) if url.endswith(".svg")}
    assert urls == SVGS_UNDER_TEXT[name]
    for url in urls:
        assert _fills((built / url.lstrip("/")).read_text(encoding="utf-8")), url


# Space's text sits on solid fills (spec.md "Variants" (Colour schemes)): the cards on
# `--card`, the hero and the footer on `--space`, over the stars, the nebula and the
# salminis' tile. Only the hero title is set over an SVG, the pink splat.
SPACE_SOLID = {"hero": "--space", "card": "--card", "base": "--space"}
SPACE_SVGS = {"/space/img/stars.svg", "/space/img/nebula.svg", "/space/img/salminis.svg", "/space/img/splat-pink.svg"}
SPACE_TITLE_SVG = "/space/img/splat-pink.svg"
AA_LARGE_TEXT = 3.0


def _rule(css: str, selector: str) -> str:
    """The declarations of every rule for exactly this selector."""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    return " ".join(block for sel, block in _rules(css) if selector in [s.strip() for s in sel.split(",")])


def test_space_text_sits_on_solid_fills(built, pages):
    page, css = pages[SPACE], _css(built, pages[SPACE])
    for cls, ground in SPACE_SOLID.items():
        assert re.search(r"(?<![\w-])background\s*:\s*var\(" + ground + r"\)\s*;", _rule(css, "." + cls)), cls
    backed = set()
    for i, (_, attrs) in enumerate(page.elements):
        if set(attrs.get("class", "").split()) & set(SPACE_SOLID):
            backed |= {j for j, _, _ in page.inside(i)}
    body = page.find("body")
    for i, tag, _ in page.inside(body):
        if tag in ("h1", "h2", "p", "li", "a") and page._text(i):
            assert i in backed, (tag, page._text(i))


def test_space_draws_only_the_svgs_the_contrast_test_knows(built, pages):
    css = _css(built, pages[SPACE])
    urls = {url for url in _css_urls(css) if url.endswith(".svg")}
    assert urls == SPACE_SVGS
    assert SPACE_TITLE_SVG in _rule(css, ".hero-title")


def test_the_space_title_meets_wcag_aa_large_text_on_every_fill_of_its_splat(built, pages):
    css = _css(built, pages[SPACE])
    assert re.search(r"color\s*:\s*var\(--ink\)", _rule(css, ".hero-title a"))
    # Large text: the title is bold and at least 2.5rem.
    assert re.search(r"font-size\s*:\s*clamp\(2\.5rem,", _rule(css, ".hero-title"))
    fills = _fills((built / SPACE_TITLE_SVG.lstrip("/")).read_text(encoding="utf-8"))
    assert fills
    ink = _rgb(variables(css)["--ink"])
    for fill in fills:
        assert contrast(ink, _rgb(fill)) >= AA_LARGE_TEXT, fill


@pytest.mark.parametrize("name", list(SVGS_UNDER_TEXT))
def test_every_text_colour_meets_wcag_aa_on_the_svgs_behind_it(built, pages, name):
    found = variables(_css(built, pages[name]))
    for url in sorted(SVGS_UNDER_TEXT[name]):
        for fill in sorted(_fills((built / url.lstrip("/")).read_text(encoding="utf-8"))):
            for text in TEXT_ON[name]:
                ratio = contrast(_rgb(found[text]), _rgb(fill))
                assert ratio >= AA_NORMAL_TEXT, f"{name} {text} on {fill} in {url}: {ratio:.2f}"


def test_the_agent_version_is_always_dark_and_nothing_moves(built, pages):
    css = _css(built, pages["agents/index.html"])
    for moving in ("animation", "transition", "@keyframes"):
        assert moving not in css, moving


def test_noir_moves_only_under_the_reduced_motion_guard(built, pages):
    # spec.md "Variants" (Motion).
    guarded, rest = _media_blocks(_css(built, pages["index.html"]), "prefers-reduced-motion: no-preference")
    assert guarded
    assert not re.search(r"(?<![\w-])(animation|animation-name|transition)\s*:", rest)


def _keyframes(css: str) -> dict[str, list[tuple[float, float]]]:
    """Each @keyframes' opacity stops, as (percent, opacity), sorted; 1 where a stop sets none."""
    found = {}
    for name, body in re.findall(r"@keyframes\s+([\w-]+)\s*{((?:[^{}]*{[^}]*})*)\s*}", css):
        stops = []
        for selectors, block in re.findall(r"([^{}]+){([^}]*)}", body):
            opacity = re.search(r"opacity:\s*([\d.]+)", block)
            for selector in selectors.split(","):
                selector = selector.strip()
                at = {"from": 0.0, "to": 100.0}.get(selector)
                stops.append((at if at is not None else float(selector.rstrip("%")),
                              float(opacity.group(1)) if opacity else 1.0))
        found[name] = sorted(stops)
    return found


def _flashes_per_second(stops: list[tuple[float, float]], seconds: float) -> float:
    # A flash is a fall in opacity followed by a rise, so count the falls over one cycle,
    # the wrap from the last stop back to the first included.
    values = [opacity for _, opacity in stops]
    falls = sum(1 for a, b in zip(values, values[1:] + values[:1]) if b < a)
    return falls / seconds


def test_the_flash_counter_counts_falls_in_opacity():
    blink = [(0, 1), (50, 1), (51, 0.25), (100, 0.25)]
    assert _flashes_per_second(blink, 0.25) == 4
    flicker = [(0, 1), (70, 1), (75, 0.2), (80, 1), (85, 0.2), (100, 1)]
    assert _flashes_per_second(flicker, 1) == 2


@pytest.mark.parametrize("name", [NOIR, SPACE])
def test_no_animation_flashes_more_than_3_times_a_second(built, pages, name):
    # WCAG 2.3.1, as spec.md "Variants" (Motion) says.
    css = _css(built, pages[name])
    frames = _keyframes(css)
    animations = re.findall(r"(?<![\w-])animation\s*:\s*([^;]+);", css)
    assert animations
    for animation in animations:
        if animation.strip() == "none":
            continue
        (name,) = [word for word in animation.split() if word in frames]
        (seconds,) = [float(t[:-1]) for t in re.findall(r"\b[\d.]+s\b", animation)]
        assert _flashes_per_second(frames[name], seconds) <= 3, animation


# A script's motion guard, spec.md "Variants" (Motion): it returns before it adds any motion
# unless the reader has no preference, a browser without matchMedia included.
SCRIPT_GUARD = re.compile(
    r'if \(!window\.matchMedia \|\| !window\.matchMedia\("\(prefers-reduced-motion: no-preference\)"\)\.matches\) return;'
)
# What board's scroll assembly may build: the frames' borders (`::before`), the bot in each
# frame (`::after`), the bots around them (`.frame-bot`) and the frame's shade, never content.
BOARD_DECORATION = re.compile(r"::before|::after|\.frame-bot\b")


def _rules(css: str) -> list[tuple[str, str]]:
    return [(selector.strip(), block) for selector, block in re.findall(r"([^{}]+){([^{}]*)}", css)]


def test_board_moves_only_under_the_reduced_motion_guard_and_builds_only_borders_and_bots(built, pages):
    css = _css(built, pages[BOARD])
    _, rest = _media_blocks(css, "prefers-reduced-motion: no-preference")
    assert not re.search(r"(?<![\w-])(animation|animation-name|transition)\s*:", rest)
    moving = 0
    for src in scripts(pages[BOARD]):
        js = (built / src.lstrip("/")).read_text(encoding="utf-8")
        set_by_script = re.findall(r'setProperty\(\s*"(--[\w-]+)"', js)
        if not set_by_script:
            continue
        moving += 1
        body = js[js.index("(function () {") + len("(function () {"):]
        first = next(line.strip() for line in body.splitlines() if line.strip() and not line.strip().startswith("//"))
        assert SCRIPT_GUARD.fullmatch(first), (src, first)
        for selector, block in _rules(css):
            for declaration in block.split(";"):
                if any(f"var({prefix}" in declaration for prefix in set_by_script):
                    prop = declaration.split(":")[0].strip()
                    assert BOARD_DECORATION.search(selector) or prop == "box-shadow", (selector, declaration)
    assert moving == 1


def test_board_hides_no_text_or_list_from_the_first_frame(built, pages):
    # Content shows at once (spec.md "Variants" (Motion)); only decoration may be clipped,
    # faded or hidden.
    for selector, block in _rules(_css(built, pages[BOARD])):
        if re.search(r"(?<![\w-])(opacity|clip-path|visibility)\s*:|display\s*:\s*none", block):
            assert BOARD_DECORATION.search(selector), selector


# What may move or hide on space: the splats, the planets, the salminis and their tile
# (`body::before`), the ink trail and the cards' drips (`::before`, `::after`), never a card.
SPACE_DECORATION = re.compile(r"::before|::after|\.splat\b|\.planet\b|\.salmini\b|\.shoal\b")
# A script that only holds data, as salminis.js holds the shoal's species, adds no motion.
DATA_SCRIPT = re.compile(r"(?://[^\n]*\n)*window\.[A-Z_]+ = {.*};\s*", re.S)


def test_space_moves_only_decoration_and_only_under_the_reduced_motion_guard(built, pages):
    guarded, rest = _media_blocks(_css(built, pages[SPACE]), "prefers-reduced-motion: no-preference")
    assert guarded
    assert not re.search(r"(?<![\w-])(animation|animation-name|transition)\s*:", rest)
    # What space.js switches on, it switches on only for readers with no motion preference.
    assert "space-js" not in rest and "landed" not in rest
    for selector, block in _rules("".join(guarded)):
        if re.search(r"(?<![\w-])(animation|animation-name|animation-duration|transition|transform)\s*:", block):
            assert SPACE_DECORATION.search(selector), selector


def test_every_space_script_returns_unless_the_reader_has_no_motion_preference(built, pages):
    moving = 0
    for src in scripts(pages[SPACE]):
        js = (built / src.lstrip("/")).read_text(encoding="utf-8")
        if DATA_SCRIPT.fullmatch(js):
            assert "function" not in js, src
            continue
        moving += 1
        body = js[js.index("(function () {") + len("(function () {"):]
        first = next(line.strip() for line in body.splitlines() if line.strip() and not line.strip().startswith("//"))
        assert SCRIPT_GUARD.fullmatch(first), (src, first)
    assert moving == 2


def test_space_hides_no_text_or_list_from_the_first_frame(built, pages):
    # The cards show at once, not waiting for their splat (spec.md "Variants" (Motion)).
    for selector, block in _rules(_css(built, pages[SPACE])):
        if re.search(r"(?<![\w-])(opacity|clip-path|visibility)\s*:|display\s*:\s*none", block):
            assert SPACE_DECORATION.search(selector), selector


def test_space_moons_count_the_live_panels_and_next_items(pages):
    now, total = _now_and_total(pages[NOIR])
    page = pages[SPACE]
    moons = page.find(class_="moons")
    found = [attrs["class"].split() for _, _, attrs in page.inside(moons) if "moon" in attrs.get("class", "").split()]
    assert (sum("painted" in m for m in found), len(found)) == (now, total)
    label = f"{now} of {total} moons painted: {now} things are out now, {total - now} come next"
    assert page.elements[moons][1]["aria-label"] == label


def _board_parts(page: Page, cls: str) -> list[dict[str, str]]:
    return [attrs for _, attrs in page.elements if cls in attrs.get("class", "").split() and "data-part" in attrs]


def test_board_parts_traces_and_blocks_name_noirs_sections(pages):
    # issue 05 "Scope": `data-part` moves to the section ids, so board.js writes them into
    # the URL, and `/board/#join` finds its block without the script.
    page = pages[BOARD]
    infos = _board_parts(page, "part-info")
    assert [(info["id"], info["data-part"]) for info in infos] == [(s, s) for s in HUMAN_SECTIONS]
    parts = _board_parts(page, "part")
    assert sorted(p["data-part"] for p in parts) == sorted(HUMAN_SECTIONS)
    assert [p["data-part"] for p in parts if "data-all" in p] == ["world"]
    traces = {t["data-part"] for t in _board_parts(page, "trace")}
    assert traces and traces <= set(HUMAN_SECTIONS) - {"world"}


def test_board_script_opens_on_world_without_a_hash(built):
    js = (built / "board" / "board.js").read_text(encoding="utf-8")
    assert re.findall(r'var DEFAULT = "([\w-]+)";', js) == ["world"]
    assert 'getAttribute("data-part")' in js


def test_board_lights_count_the_live_panels_and_next_items(pages):
    # The owner edits noir by hand, and genboard.py draws the lights from it.
    now, total = _now_and_total(pages[NOIR])
    page = pages[BOARD]
    (lights,) = [i for i, (_, attrs) in enumerate(page.elements)
                 if "part" in attrs.get("class", "").split() and attrs.get("data-part") == "next"]
    leds = [attrs["class"].split() for _, tag, attrs in page.inside(lights) if "led" in attrs.get("class", "").split()]
    assert (sum("on" in led for led in leds), len(leds)) == (now, total)


def _world_version() -> str:
    # The major is the public copy's server path, `/v1`, and the date is `info.version`.
    source = generate_public_openapi.load_yaml(generate_public_openapi.SOURCE)
    public = generate_public_openapi.build_public(
        source, generate_public_openapi.load_yaml(generate_public_openapi.PUBLIC_SECURITY), CONTACT
    )
    return f"{public['servers'][0]['url'].strip('/')} {source['info']['version']}"


def test_board_prints_the_world_version_of_the_api_description():
    page = Page((REPO_WEBSITE / BOARD).read_text(encoding="utf-8"))
    assert page.text_of("text", **{"class": "board-version"}) == f"botrefugium {_world_version()}"


def _shape_under(page: Page, svg: int, text: int) -> str:
    """The fill of the topmost solid shape before the `<text>` at `text` under its anchor."""
    attrs = page.elements[text][1]
    x, y = float(attrs["x"]), float(attrs["y"]) - float(attrs["font-size"]) / 2
    fill = None
    for i, tag, shape in page.inside(svg):
        if i >= text:
            break
        if "fill-opacity" in shape or "hit" in shape.get("class", "").split():
            continue
        if tag == "path" and shape.get("fill"):
            fill = shape["fill"]  # the board itself, under everything
        elif tag == "rect" and shape.get("fill"):
            left, top = float(shape["x"]), float(shape["y"])
            if left <= x <= left + float(shape["width"]) and top <= y <= top + float(shape["height"]):
                fill = shape["fill"]
    assert fill, attrs
    return fill


def test_board_svg_text_meets_wcag_aa_on_what_it_sits_on(pages):
    page = pages[BOARD]
    svg = page.find("svg", class_="board-svg")
    texts = [i for i, tag, _ in page.inside(svg) if tag == "text"]
    assert texts
    for i in texts:
        fill = page.elements[i][1]["fill"]
        ratio = contrast(_rgb(fill), _rgb(_shape_under(page, svg, i)))
        assert ratio >= AA_NORMAL_TEXT, (page._text(i), fill, ratio)


def test_the_agent_overlay_is_capped_at_8_percent_by_one_variable(built, pages):
    css = _css(built, pages["agents/index.html"])
    found = variables(css)
    assert float(found["--overlay-max"]) <= OVERLAY_CAP
    overlay = re.search(r"body::before\s*{([^}]*)}", css).group(1)
    assert re.search(r"opacity:\s*var\(--overlay-max\)\s*;", overlay)
    assert re.search(r"z-index:\s*-1\s*;", overlay)
    # Its colours come only from the overlay variables, which the contrast test blends in.
    assert not re.search(r"#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(", overlay)
    assert set(re.findall(r"var\((--[\w-]+)\)", overlay)) == {"--overlay-max", "--scan", "--grain"}


@pytest.mark.parametrize("name", PAGES)
def test_every_text_colour_meets_wcag_aa_plain_and_under_the_full_overlay(built, pages, name):
    found = variables(_css(built, pages[name]))
    for text, grounds in TEXT_ON[name].items():
        for ground in grounds:
            bg = _rgb(found[ground])
            backgrounds = {"plain": bg}
            if "--overlay-max" in found:
                cap = float(found["--overlay-max"])
                for ink in ("--scan", "--grain"):
                    backgrounds[f"under {ink}"] = _blend(bg, _rgb(found[ink]), cap)
            for where, background in backgrounds.items():
                ratio = contrast(_rgb(found[text]), background)
                assert ratio >= AA_NORMAL_TEXT, f"{name} {text} on {ground} {where}: {ratio:.2f}"


# 360 px less the 1rem gutters on each side.
PHONE_WIDTH_PX = 360
PHONE_CONTENT_PX = 328
REM_PX = 16


def _max_width_blocks(css: str) -> list[tuple[float, str]]:
    """Each `@media (max-width: <n>rem) { ... }` as (n, body), by matching braces."""
    found = []
    for match in re.finditer(r"@media\s*\(\s*max-width\s*:\s*([\d.]+)rem\s*\)\s*{", css):
        depth, i = 1, match.end()
        while depth:
            depth += {"{": 1, "}": -1}.get(css[i], 0)
            i += 1
        found.append((float(match.group(1)), css[match.end() : i - 1]))
    return found


@pytest.mark.parametrize("name", PAGES)
def test_the_layout_is_one_column_that_fits_a_360_px_phone(built, pages, name):
    css = _css(built, pages[name])
    assert not re.search(r"(?<![\w-])(columns|column-count|float)\s*:", css)
    # A grid may lay panels side by side on wider screens, as long as two of its smallest
    # columns can't fit a phone (spec.md "Variants" (Layout)).
    # Or it may set fixed columns that a `max-width` query of at least 360 px sets back to one
    # (space's two columns).
    collapsed = set()
    for width, body in _max_width_blocks(css):
        if width * REM_PX >= PHONE_WIDTH_PX:
            collapsed |= {sel for sel, block in _rules(body)
                          if re.search(r"grid-template-columns\s*:\s*1fr\s*;", block)}
    for selector, block in _rules(css):
        fixed = re.search(r"(?<![\w-])grid-template-columns\s*:\s*((?:[\d.]+fr\s*){2,});", block)
        if fixed:
            assert selector in collapsed, selector
            css = css.replace(fixed.group(0), "")
    for value in re.findall(r"(?<![\w-])grid-template-columns\s*:\s*([^;]+);", css):
        if value.strip() == "1fr":
            continue
        smallest = re.fullmatch(r"repeat\(\s*auto-(?:fit|fill)\s*,\s*minmax\(\s*([\d.]+)rem\s*,\s*1fr\s*\)\s*\)", value.strip())
        assert smallest, value
        assert 2 * float(smallest.group(1)) * REM_PX > PHONE_CONTENT_PX, value
    for prop, px in re.findall(r"(?<![\w-])((?:min-)?width)\s*:\s*(\d+)px", css):
        assert int(px) <= PHONE_CONTENT_PX, f"{prop}: {px}px"


# A variable a page sets per element in a `style` attribute, standing for one of its text
# colours, and the ground that text sits on: space's `--c`, each stop's ink on its card.
PER_ELEMENT = {SPACE: {"--c": "--card"}}


@pytest.mark.parametrize("name", PAGES)
def test_text_colours_are_only_set_through_the_checked_variables(built, pages, name):
    # A `color:` with a literal, or a variable the table doesn't list, would escape the
    # contrast test.
    css = _css(built, pages[name])
    for value in re.findall(r"(?<![\w-])color\s*:\s*([^;]+);", css):
        used = re.fullmatch(r"var\((--[\w-]+)\)", value.strip())
        assert used and (used.group(1) in TEXT_ON[name] or used.group(1) in PER_ELEMENT.get(name, {})), value


@pytest.mark.parametrize("name", list(PER_ELEMENT))
def test_a_per_element_text_colour_names_a_checked_variable(pages, name):
    for variable, ground in PER_ELEMENT[name].items():
        values = [m for _, attrs in pages[name].elements
                  for m in re.findall(re.escape(variable) + r"\s*:\s*([^;]+)", attrs.get("style", ""))]
        assert values, variable
        for value in values:
            used = re.fullmatch(r"var\((--[\w-]+)\)", value.strip())
            assert used and ground in TEXT_ON[name].get(used.group(1), []), value


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
    join = pages["agents/index.html"].section("join")
    steps = [
        "curl https://api.example.test/v1/world/constitution",
        "curl https://api.example.test/v1/openapi.yaml",
        "curl https://api.example.test/v1/world/genesis-code",
        "curl -X POST https://api.example.test/v1/identity/birth",
        "curl https://api.example.test/v1/identity/self",
    ]
    found = [join.find(step) for step in steps]
    assert -1 not in found, dict(zip(steps, found))
    assert found == sorted(found)
    for needed in ("Proof: ", '"genesis_code"', "Authorization: Bearer ", "the owner's posts"):
        assert needed in join, needed


def test_the_agent_join_steps_name_the_birth_limits(pages):
    # spec.md "How to join": one birth per IP, or `409` `too-soon`, and a lost answer burns the code.
    join = pages["agents/index.html"].section("join")
    for needed in ("409 try-later", "409 too-soon", "One birth per IP address", "so is the code"):
        assert needed in join, needed


def _items(page: Page, section_id: str, list_class: str) -> int:
    section = page.find(id=section_id)
    lists = [i for i, tag, attrs in page.inside(section) if tag == "ul" and attrs.get("class") == list_class]
    assert len(lists) == 1, list_class
    return sum(1 for _, tag, _ in page.inside(lists[0]) if tag == "li")


def _now_and_total(noir: Page) -> tuple[int, int]:
    """What is out now, noir's panels marked `data-live`, and that plus the items of #next."""
    live = [attrs.get("id") for _, attrs in noir.elements if "data-live" in attrs]
    # The markers are the one source; they only have to sit on real sections.
    assert live and set(live) <= set(HUMAN_SECTIONS) - {"next"}
    return len(live), len(live) + _items(noir, "next", "next")


NOIR_PROBES = {
    "a marker of any value, on any tag": "".join([
        '<details id="identity" data-live="true"></details>',
        '<div id="homes" data-live></div>',
        '<section id="next"><div><ul class="next"><li>a</li><li>b</li></ul></div></section>',
    ]),
    "a list outside #next doesn't count": "".join([
        '<div id="friends" data-live></div>',
        '<details id="next"><ul class="next"><li>a</li></ul></details>',
        '<ul class="next"><li>z</li></ul>',
    ]),
    "an empty #next list": "".join([
        '<div id="friends" data-live></div>',
        '<section id="next"><ul class="next"></ul></section>',
    ]),
    "a marker without an id": "".join([
        '<div id="friends" data-live></div><p data-live></p>',
        '<section id="next"><ul class="next"><li>a</li></ul></section>',
    ]),
    "a marker on #next itself": "".join([
        '<div id="friends" data-live></div>',
        '<section id="next" data-live><ul class="next"><li>a</li></ul></section>',
    ]),
    "no marker at all": '<section id="next"><ul class="next"><li>a</li></ul></section>',
    "two lists in #next": "".join([
        '<div id="friends" data-live></div>',
        '<section id="next">',
        '<ul class="next"><li>a</li></ul>',
        '<ul class="next"><li>b</li></ul>',
        '</section>',
    ]),
    "a list nested in the list": "".join([
        '<div id="friends" data-live></div>',
        '<section id="next">',
        '<ul class="next"><li>a<ul class="next"><li>b</li></ul></li></ul>',
        '</section>',
    ]),
}


# The probes both copies refuse, each with the reason noircounts gives; the rest they count.
PROBE_REFUSALS = {
    "a marker without an id": "a data-live marker needs a section id, found 1 without",
    "a marker on #next itself": "a data-live marker needs a section id, found 1 without",
    "no marker at all": "found no data-live panel",
    "two lists in #next": 'needs one <ul class="next">, found 2',
    "a list nested in the list": 'needs one <ul class="next">, found 2',
}


@pytest.mark.parametrize("probe", NOIR_PROBES)
def test_the_board_counts_noir_by_the_page_tests_rule(probe):
    # genboard.py draws the lights with noircounts, a second copy of `_now_and_total`'s rule:
    # run both on the same pages, and they agree or both refuse.
    html = NOIR_PROBES[probe]
    refusal = PROBE_REFUSALS.get(probe)
    if refusal:
        with pytest.raises(AssertionError):
            _now_and_total(Page(html))
        with pytest.raises(SystemExit, match=re.escape(refusal)):
            noircounts.count_live_and_next(html)
        return
    now, total = _now_and_total(Page(html))
    # noircounts gives (live, coming), the tests (now, total): compare like with like.
    assert noircounts.count_live_and_next(html) == (now, total - now)


def test_the_board_counts_noir_itself_by_the_page_tests_rule(pages):
    html = (REPO_WEBSITE / NOIR).read_text(encoding="utf-8")
    now, total = _now_and_total(Page(html))
    assert noircounts.count_live_and_next(html) == (now, total - now)


def test_noir_meter_counts_the_live_panels_and_next_items(pages):
    # The owner edits noir by hand, so the meter must follow its live panels and #next.
    page = pages["index.html"]
    now, total = _now_and_total(page)
    meter = page.find(class_="hud-meter")
    segments = [attrs["class"].split() for _, tag, attrs in page.inside(meter) if "seg" in attrs.get("class", "").split()]
    assert (sum("on" in s for s in segments), len(segments)) == (now, total)
    assert page.elements[meter][1]["aria-label"] == f"Now: {now} of {total} features are live"
    assert f"{now}/{total}" in page.text_of("div", **{"class": "hud-meter"})


def test_noir_header_shows_the_world_version_of_the_api_description():
    page = Page((REPO_WEBSITE / "index.html").read_text(encoding="utf-8"))
    assert page.text_of("span", **{"class": "hud-version"}) == "World " + _world_version().replace(" ", " · ")


FEDERATION_STEPS = "https://github.com/gratinierer/sfab/blob/main/docs/federation.md"
REPO = "https://github.com/gratinierer/sfab"


def test_the_agent_version_links_the_federation_steps(pages):
    hrefs = [a["href"] for a in pages["agents/index.html"].all("a")]
    assert FEDERATION_STEPS in hrefs


@pytest.mark.parametrize("name", PAGES)
def test_open_source_links_the_repo_and_names_the_mit_license(pages, name):
    hrefs = [a["href"] for a in pages[name].all("a")]
    assert REPO in hrefs
    assert "MIT" in pages[name].section("open-source")


@pytest.mark.parametrize("name", PAGES)
def test_no_page_speaks_of_cost(built, name):
    # issue 08: the world runs free for as long as we can afford it; "cost" in any form is out.
    # Any word starting "cost" counts as a form of it; only "costume", a different word, is let through.
    assert not re.search(r"\bcost(?!ume)\w*", (built / name).read_text(encoding="utf-8"), re.I)


def test_noir_uses_no_game_words(built):
    # issue 08: noir is not a game.
    text = (built / NOIR).read_text(encoding="utf-8").lower()
    for phrase in ("Choose your mission", "Press start", "Continue?"):
        assert phrase.lower() not in text, phrase


@pytest.mark.parametrize("name", PAGES)
def test_every_sfab_in_the_text_links_the_repo(built, name):
    # The owner's rule of 2026-11-22: wherever the text names sfab, it links the repo. Code,
    # the board's chip label and the links themselves are left out before looking.
    html = (built / name).read_text(encoding="utf-8")
    body = html[html.index("<body"):]
    for pattern in (rf'<a href="{REPO}">sfab</a>', r"<svg.*?</svg>", r"<pre.*?</pre>", r"<code.*?</code>",
                    r"<a [^>]*>.*?</a>", r"<[^>]+>"):
        body = re.sub(pattern, " ", body, flags=re.S)
    assert not re.search(r"\bsfab\b", body)


@pytest.mark.parametrize("name", PAGES)
def test_no_page_says_a_reader_has_an_agent(built, name):
    # The owner's rule of 2026-11-22: nobody owns an agent; a reader knows one.
    assert not re.search(r"\b(have|own|owns|your) (an |a )?(agent|bot)\b", (built / name).read_text(encoding="utf-8"), re.I)
