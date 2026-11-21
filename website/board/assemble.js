// Assembles each .frame of the board page as the reader scrolls it into view, and takes it
// apart again on the way back up: the higher a frame's top has risen in the viewport, the more
// of its border and bots is built. Its content shows at once, from the first frame
// (.stories/10-website/spec.md "Variants" (Motion)). The script only sets custom properties
// on each frame and its bots (see board.css, ".frame::before"); without it, or for readers
// who ask for less motion, frames are simply shown finished.
(function () {
  if (!window.matchMedia || !window.matchMedia("(prefers-reduced-motion: no-preference)").matches) return;

  var EDGE_STEPS = 24; // pixel steps for the whole border
  var BOT_ROWS = 12; // a bot is 12 pixels tall and builds up one row at a time
  var SHADE_STEPS = 4;
  // Where a frame's top edge is, as a share of the viewport height from the top, when it
  // starts to assemble and when it is finished.
  var START_AT = 0.9;
  var DONE_AT = 0.4;

  function clamp(x) {
    return x < 0 ? 0 : x > 1 ? 1 : x;
  }

  function quantize(x, steps) {
    return Math.floor(x * steps) / steps;
  }

  // Spots for the bots around a frame, beside it or below it, where the next frame or the
  // page hides part of them. None go above a frame: the frame before it would not cover
  // them. Each frame takes three spots, a different set each time.
  var SPOTS = [
    { left: "-26px", top: "22%", width: 33, rotate: -6 },
    { left: "calc(100% - 6px)", top: "48%", width: 22, rotate: 8 },
    { left: "12%", top: "calc(100% - 4px)", width: 22, rotate: 0 },
    { left: "calc(100% - 4px)", top: "calc(100% - 30px)", width: 33, rotate: -10 },
    { left: "58%", top: "calc(100% - 2px)", width: 22, rotate: 4 }
  ];

  function addBot(frame, spot) {
    var bot = document.createElement("span");
    bot.className = "frame-bot";
    bot.setAttribute("aria-hidden", "true");
    bot.style.left = spot.left;
    bot.style.top = spot.top;
    bot.style.width = spot.width + "px";
    bot.style.height = Math.round((spot.width * 12) / 11) + "px";
    bot.style.transform = "rotate(" + spot.rotate + "deg)";
    frame.insertBefore(bot, frame.firstChild); // first, so :last-child stays the text
    return bot;
  }

  // Returns the frame's bots, each with the point (0 to 1) where it starts to appear.
  function addBots(frame, index) {
    var bots = [];
    for (var k = 0; k < 3; k++) {
      bots.push({ el: addBot(frame, SPOTS[(index + k * 2) % SPOTS.length]), from: 0.15 + k * 0.2 });
    }
    return bots;
  }

  function start() {
    var frames = Array.prototype.slice.call(document.querySelectorAll(".frame"));
    var frameBots = frames.map(addBots);
    document.documentElement.classList.add("bots-around");
    var queued = false;

    function update() {
      queued = false;
      var root = document.documentElement;
      var vh = window.innerHeight || root.clientHeight;
      var scrollY = window.pageYOffset || root.scrollTop;
      var maxScroll = Math.max(0, root.scrollHeight - vh);
      frames.forEach(function (frame, index) {
        var rect = frame.getBoundingClientRect();
        // 0 while the frame's top is below START_AT, 1 once it has risen to DONE_AT. A frame
        // near the end of the page may never rise that far, so it is finished by the time
        // the page is scrolled to the bottom.
        var begin = vh * START_AT;
        var topAtBottom = rect.top - (maxScroll - scrollY);
        var done = Math.max(vh * DONE_AT, topAtBottom);
        var p = begin - done < 1 ? (rect.top <= done ? 1 : 0) : clamp((begin - rect.top) / (begin - done));
        var draw = quantize(clamp(p / 0.7), EDGE_STEPS);
        var style = frame.style;
        var edges = [];
        for (var i = 0; i < 4; i++) {
          edges[i] = clamp(draw * 4 - i);
          style.setProperty("--edge-" + (i + 1), edges[i]);
        }
        // Clockwise from the top left: each corner appears as the edge before it reaches it.
        style.setProperty("--corner-1", draw > 0 ? 1 : 0);
        for (var c = 1; c < 4; c++) {
          style.setProperty("--corner-" + (c + 1), edges[c - 1] >= 1 ? 1 : 0);
        }
        // The bot in the frame builds up while the edges draw; the bots around it pop up
        // one after another over the same stretch.
        style.setProperty("--bot", quantize(clamp((p - 0.1) / 0.5), BOT_ROWS));
        frameBots[index].forEach(function (bot) {
          bot.el.style.setProperty("--bot", quantize(clamp((p - bot.from) / 0.2), BOT_ROWS));
        });
        style.setProperty("--shade", quantize(clamp((p - 0.6) / 0.4), SHADE_STEPS));
      });
    }

    function queue() {
      if (queued) return;
      queued = true;
      window.requestAnimationFrame(update);
    }

    update();
    window.addEventListener("scroll", queue, { passive: true });
    window.addEventListener("resize", queue);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
