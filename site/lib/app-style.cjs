// The app's stylesheet (desktop/ui/style.css), made safe to load in the website.
//
// The app styles a whole window: html, body, every button. On the site the same rules may only
// reach the app's interface, so every rule is moved under one class, `.cf`. The app's tokens
// are also published to the whole site as `--app-*`, which is where the site gets its accent
// from: the colours are written once, in desktop/ui/style.css.
//
// This file is a webpack loader (next.config.ts) and the function it runs.

const postcss = require("postcss");

const SCOPE = ".cf";

/** One selector of the app's, as it applies inside the scope. Null when it has no place there. */
function scoped(selector) {
  const s = selector.trim();
  // The page itself belongs to the site, and the other looks (?look= in the lab) are not used.
  if (/^html\b/.test(s) || /^:root\[/.test(s)) return null;
  if (s === ":root" || s === "body") return SCOPE;
  if (s === "*") return `${SCOPE}, ${SCOPE} *`;
  return `${SCOPE} ${s}`;
}

function transform(css) {
  const root = postcss.parse(css);
  const tokens = [];

  root.walkRules((rule) => {
    // The steps of an animation are not selectors.
    if (rule.parent.type === "atrule" && /keyframes$/.test(rule.parent.name)) return;
    if (rule.selector.trim() === ":root") {
      rule.walkDecls(/^--/, (decl) => {
        tokens.push(`  --app-${decl.prop.slice(2)}: ${decl.value.replace(/var\(--/g, "var(--app-")};`);
      });
    }
    const selectors = rule.selectors.map(scoped);
    // A rule about the page (html, body together) is dropped whole.
    if (selectors.includes(null)) return void rule.remove();
    rule.selectors = selectors;
  });

  return `:root {\n${tokens.join("\n")}\n}\n${root.toString()}`;
}

module.exports = function appStyleLoader(source) {
  return transform(source);
};
module.exports.transform = transform;
