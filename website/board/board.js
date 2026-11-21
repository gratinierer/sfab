// The board page: clicking a part of the board (or pressing Enter or Space on it) shows that
// part's info in the box below, hides the others and lights up the part's lines. The chosen
// part's section id is kept in the URL (#world, #join, ...), the same anchors as the noir
// page's, so a link can point at one; with no hash the page shows #world. Without this
// script every block is listed (.stories/10-website/spec.md "Variants" (Board)).
(function () {
  var DEFAULT = "world";
  var parts = Array.prototype.slice.call(document.querySelectorAll(".part"));
  var infos = Array.prototype.slice.call(document.querySelectorAll(".part-info"));
  var traces = Array.prototype.slice.call(document.querySelectorAll(".trace"));
  if (!parts.length || !infos.length) return;
  document.documentElement.classList.add("board-js");

  function exists(name) {
    return infos.some(function (info) { return info.getAttribute("data-part") === name; });
  }

  function select(name) {
    parts.forEach(function (part) {
      part.setAttribute("aria-pressed", part.getAttribute("data-part") === name ? "true" : "false");
    });
    infos.forEach(function (info) {
      info.hidden = info.getAttribute("data-part") !== name;
    });
    // A part marked data-all (the board's processor, where every line ends) lights them all.
    var all = parts.some(function (part) {
      return part.getAttribute("data-part") === name && part.hasAttribute("data-all");
    });
    traces.forEach(function (trace) {
      trace.classList.toggle("is-on", all || trace.getAttribute("data-part") === name);
    });
    window.dispatchEvent(new Event("resize")); // the info box changed size: re-place its bots
  }

  parts.forEach(function (part) {
    var name = part.getAttribute("data-part");
    function choose() {
      select(name);
      if (history.replaceState) history.replaceState(null, "", "#" + name);
    }
    part.addEventListener("click", choose);
    part.addEventListener("keydown", function (event) {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        choose();
      }
    });
  });

  function fromHash() {
    var name = location.hash.slice(1);
    select(exists(name) ? name : DEFAULT);
  }

  window.addEventListener("hashchange", fromHash);
  fromHash();
})();
