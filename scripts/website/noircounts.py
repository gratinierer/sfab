"""Count what is out now and what comes next on the noir page, `website/index.html`.

The board's lights (`genboard.py`) are drawn from these counts. They follow the page tests'
rules, `_now_and_total` and `_items` in `cdk/tests/test_website_pages.py`, and a test there
runs this module against them, so the two copies of the rule can't drift apart. No side
effects: importing it reads and writes nothing.
"""
from html.parser import HTMLParser


class _NoirCounts(HTMLParser):
    """Counts what is out now and what comes next on noir, by the page tests' rules
    (`_now_and_total`, `_items`): any element with a `data-live` attribute, whatever its value,
    and the <li> items of the one `<ul class="next">` inside the element with `id="next"`,
    whatever its tag. The results are `live`, `coming` and `next_lists`; the underscored
    fields are the walk's own state."""

    def __init__(self) -> None:
        super().__init__()
        self.live = 0
        self.coming = 0
        self.next_lists = 0     # the `<ul class="next">` lists found inside #next
        self._next_tag = None   # #next's own tag, once it opens
        self._next_depth = 0    # open elements of that tag inside #next, #next itself included
        self._list_depth = 0    # open <ul> inside a `<ul class="next">`, itself included

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "data-live" in attrs:
            self.live += 1
        if self._next_depth and tag == self._next_tag:
            self._next_depth += 1
        elif not self._next_depth and attrs.get("id") == "next":
            self._next_tag, self._next_depth = tag, 1
        if tag == "ul" and self._next_depth and attrs.get("class") == "next":
            self.next_lists += 1    # nested ones too, as `_items` counts every one
        if tag == "ul" and (self._list_depth or (self._next_depth and attrs.get("class") == "next")):
            self._list_depth += 1
        if tag == "li" and self._list_depth:
            self.coming += 1

    def handle_endtag(self, tag):
        if tag == "ul" and self._list_depth:
            self._list_depth -= 1
        if tag == self._next_tag and self._next_depth:
            self._next_depth -= 1


def count_live_and_next(html: str) -> tuple[int, int]:
    """Noir's panels marked `data-live`, and the items of its #next list."""
    counts = _NoirCounts()
    counts.feed(html)
    counts.close()
    if counts.next_lists != 1:
        raise SystemExit(f'genboard: noir\'s #next needs one <ul class="next">, found {counts.next_lists}')
    if not counts.live or not counts.coming:
        raise SystemExit("genboard: found no data-live panel or no #next item on noir")
    return counts.live, counts.coming
