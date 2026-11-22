// The space page: each planet's splat lands when its stop comes into view, and again when
// the planet is clicked; the cards show at once either way. Without this script, or unless
// the reader has no motion preference, everything is simply shown.
(function () {
  if (!window.matchMedia || !window.matchMedia("(prefers-reduced-motion: no-preference)").matches) return;
  if (!("IntersectionObserver" in window)) return;
  document.documentElement.classList.add("space-js");

  function start() {
    var stops = Array.prototype.slice.call(document.querySelectorAll(".stop"));
    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (!entry.isIntersecting) return;
          entry.target.classList.add("landed");
          observer.unobserve(entry.target);
        });
      },
      { rootMargin: "0px 0px -20% 0px" }
    );
    stops.forEach(function (stop) {
      observer.observe(stop);
      var planet = stop.querySelector(".planet");
      if (!planet) return;
      planet.addEventListener("click", function () {
        // Restart the splat: drop the class, force a reflow, add it back.
        stop.classList.remove("landed");
        void stop.offsetWidth;
        stop.classList.add("landed");
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
