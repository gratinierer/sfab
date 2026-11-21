"""The website's pages, as the generate step builds them: the noir page at `/`, the board page
at `/board/` and the agent version at `/agents/`, from .stories/10-website/spec.md "Design and
content", "Mobile and access" and "Variants", which wins where they disagree, and its issues
01, 04 and 05.

Each page is parsed with the standard library's HTML parser, so these tests need no browser.
"""

import re
from html.parser import HTMLParser
from pathlib import Path

import pytest

import generate_public_openapi
import generate_website

REPO_WEBSITE = Path(__file__).resolve().parents[2] / "website"

PAGES = ("index.html", "board/index.html", "agents/index.html")
NOIR, BOARD, AGENT = PAGES
# spec.md "Variants" (Sections): the human-side pages use noir's seven, the agent version
# keeps its five, each in its own order.
HUMAN_SECTIONS = ["world", "join", "identity", "constitution", "proof-of-work", "now-and-next", "open-source"]
AGENT_SECTIONS = ["what", "join", "now-and-next", "proof-of-work", "open-source"]
SECTIONS = {NOIR: HUMAN_SECTIONS, BOARD: HUMAN_SECTIONS, AGENT: AGENT_SECTIONS}
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

    def paragraphs_and_lists(self, section_id: str) -> list[tuple[str, str, list[str]]]:
        """The section's paragraphs and lists, each as (tag, text, link targets), leaving out
        its headings, noir's themed captions and a nested `<details>` (noir's worked
        example), which spec.md "Variants" (Same content) lets each page theme or hold alone."""
        blocks, skip_to = [], -1
        for i, tag, attrs in self.inside(self._index_of(section_id)):
            if i < skip_to:
                continue
            if tag == "details" or (tag == "p" and attrs.get("class") == "caption"):
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


@pytest.mark.parametrize("section_id", HUMAN_SECTIONS)
def test_board_carries_noirs_paragraphs_and_lists_word_for_word(pages, section_id):
    # spec.md "Variants" (Same content): noir's text is the source; only headings may differ.
    noir = pages[NOIR].paragraphs_and_lists(section_id)
    assert noir
    assert pages[BOARD].paragraphs_and_lists(section_id) == noir


def test_board_links_the_worked_example_on_noir(pages):
    links = [href for _, _, hrefs in pages[BOARD].paragraphs_and_lists("proof-of-work") for href in hrefs]
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


@pytest.mark.parametrize(
    ("name", "home", "switches"),
    [
        (NOIR, "/", [("/agents/", "I'm an agent →")]),
        (BOARD, "/board/", [("/", "Human version"), ("/agents/", "I'm an agent →")]),
        (AGENT, "/agents/", [("/", "← human version")]),
    ],
)
def test_the_header_holds_the_wordmark_and_the_switch_links_to_the_other_versions(
    pages, name, home, switches
):
    # spec.md "Variants": a link home to the page named botrefugium, the wordmark image or
    # themed text, and, until issue 07's four-way switch, each page's own switch links.
    page = pages[name]
    header = next(i for i, (tag, _) in enumerate(page.elements) if tag == "header")
    inside = page.inside(header)
    links = [(i, attrs) for i, tag, attrs in inside if tag == "a"]

    def name_of(i: int) -> str:
        images = [attrs.get("alt", "") for _, tag, attrs in page.inside(i) if tag == "img"]
        return " ".join(("".join(page.text.get(i, [])) + " ".join(images)).split())

    assert [(attrs["href"], name_of(i)) for i, attrs in links if attrs.get("class") != "switch"] == [
        (home, "botrefugium")
    ]
    assert [(attrs["href"], name_of(i)) for i, attrs in links if attrs.get("class") == "switch"] == switches


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


# spec.md "Variants" (JavaScript): board may load scripts from its own folder only; noir and
# the agent version have none.
SCRIPT_FOLDER = {BOARD: "/board/"}


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


# What board may load from the site root; everything else it uses lives in `/board/`
# (issue 05 "Scope").
SHARED = {"/shelter-icon.svg", "/shelter-wordmark.svg", "/fonts/silkscreen-latin-400-normal.woff2"}


def test_board_loads_nothing_from_the_site_root_but_the_icon_the_wordmark_and_the_font(built, pages):
    page = pages[BOARD]
    loaded = [link["href"] for link in page.all("link")] + [img["src"] for img in page.all("img")]
    loaded += scripts(page)
    for href in stylesheets(page):
        css = (built / href.lstrip("/")).read_text(encoding="utf-8")
        loaded += re.findall(r"url\(\s*['\"]?([^'\")]+)", css)
    assert loaded
    for url in loaded:
        assert url.startswith("/board/") or url in SHARED, url


@pytest.mark.parametrize("name", PAGES)
def test_a_stylesheet_imports_nothing_and_its_urls_are_files_of_the_site(built, pages, name):
    for href in stylesheets(pages[name]):
        css = (built / href.lstrip("/")).read_text(encoding="utf-8")
        assert "@import" not in css, href
        for url in re.findall(r"url\(\s*['\"]?([^'\")]+)", css):
            assert _is_local(url), (href, url)
            assert (built / url.lstrip("/")).is_file(), (href, url)


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


SCHEME = {NOIR: "dark", BOARD: "light", AGENT: "dark"}


@pytest.mark.parametrize("name", PAGES)
def test_each_page_has_one_fixed_scheme(built, pages, name):
    # spec.md "Variants" (Colour schemes): noir dark, board light lilac, agent dark, none
    # following the browser.
    css = _css(built, pages[name])
    assert "prefers-color-scheme" not in css
    assert re.search(r"color-scheme:\s*" + SCHEME[name] + r"\s*;", css)
    for text, grounds in TEXT_ON[name].items():
        for ground in grounds:
            luminance = _luminance(_rgb(variables(css)[ground]))
            if SCHEME[name] == "light":
                assert luminance > 0.7, ground
            else:
                assert luminance < 0.05 or ground == "--signal", ground


def _fills(svg: str) -> set[str]:
    return set(re.findall(r'fill="(#[0-9a-fA-F]{6})"', svg))


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


def test_no_noir_animation_flashes_more_than_3_times_a_second(built, pages):
    # WCAG 2.3.1, as spec.md "Variants" (Motion) says.
    css = _css(built, pages["index.html"])
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


def test_board_lights_count_the_now_and_next_items(pages):
    # The owner edits noir's #now-and-next by hand, and genboard.py draws the lights from it.
    noir = pages[NOIR]
    now = _items(noir, "now-and-next", "now")
    total = now + _items(noir, "now-and-next", "next")
    page = pages[BOARD]
    (lights,) = [i for i, (_, attrs) in enumerate(page.elements)
                 if "part" in attrs.get("class", "").split() and attrs.get("data-part") == "now-and-next"]
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
    (svg,) = [i for i, (tag, attrs) in enumerate(page.elements) if tag == "svg" and attrs.get("class") == "board-svg"]
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
PHONE_CONTENT_PX = 328
REM_PX = 16


@pytest.mark.parametrize("name", PAGES)
def test_the_layout_is_one_column_that_fits_a_360_px_phone(built, pages, name):
    css = _css(built, pages[name])
    assert not re.search(r"(?<![\w-])(columns|column-count|float)\s*:", css)
    # A grid may lay panels side by side on wider screens, as long as two of its smallest
    # columns can't fit a phone (spec.md "Variants" (Layout)).
    for value in re.findall(r"(?<![\w-])grid-template-columns\s*:\s*([^;]+);", css):
        smallest = re.fullmatch(r"repeat\(\s*auto-(?:fit|fill)\s*,\s*minmax\(\s*([\d.]+)rem\s*,\s*1fr\s*\)\s*\)", value.strip())
        assert smallest, value
        assert 2 * float(smallest.group(1)) * REM_PX > PHONE_CONTENT_PX, value
    for prop, px in re.findall(r"(?<![\w-])((?:min-)?width)\s*:\s*(\d+)px", css):
        assert int(px) <= PHONE_CONTENT_PX, f"{prop}: {px}px"


@pytest.mark.parametrize("name", PAGES)
def test_text_colours_are_only_set_through_the_checked_variables(built, pages, name):
    # A `color:` with a literal, or a variable the table doesn't list, would escape the
    # contrast test.
    css = _css(built, pages[name])
    for value in re.findall(r"(?<![\w-])color\s*:\s*([^;]+);", css):
        used = re.fullmatch(r"var\((--[\w-]+)\)", value.strip())
        assert used and used.group(1) in TEXT_ON[name], value


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
    section = next(i for i, (_, attrs) in enumerate(page.elements) if attrs.get("id") == section_id)
    lists = [i for i, tag, attrs in page.inside(section) if tag == "ul" and attrs.get("class") == list_class]
    assert len(lists) == 1, list_class
    return sum(1 for _, tag, _ in page.inside(lists[0]) if tag == "li")


def test_noir_meter_counts_the_now_and_next_items(pages):
    # The owner edits #now-and-next by hand, so the meter must follow its items.
    page = pages["index.html"]
    now = _items(page, "now-and-next", "now")
    total = now + _items(page, "now-and-next", "next")
    meter = next(i for i, (_, attrs) in enumerate(page.elements) if attrs.get("class") == "hud-meter")
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
