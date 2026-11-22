# Add the four-way switch and list every page

Blocked by: 05, 06

`spec.md` "Variants", "Four pages, one switch", "Same content" and "Discovery files".

## Scope

- Every page's header gets the same switch of four links: "I'm human" to `/`, "I'm nerd" to `/board/`,
  "I'm 11" to `/space/` and "I'm agent" to `/agents/`, the current one with `aria-current="page"`. It replaces the
  single switch link, noir's included, and the scratch pages' footer links, and the header test checks the four
  links instead of one.
- `website/sitemap.xml` lists `/`, `/board/`, `/space/` and `/agents/`. `website/llms.txt` is unchanged.
- A test compares the paragraphs and lists of noir, board and space section by section and finds them identical.
- `spec.md` "Design and content" bullets that "Variants" amends get a pointer to it, so a reader of either finds
  the other.
- gratinierer/sfab#13 lists all four pages for the owner's sign-off.

## Done when

- A test finds the same four-link switch, in the same order and with those labels, in every page's header, and
  `aria-current="page"` on exactly the current page's link.
- The identical-text test passes.
- On a deploy: every link on all four pages and in `/llms.txt` resolves, apart from the GitHub links, under the
  same exception as ticket 03.
