// The whole "framework": make an element, set what it needs.

type Props = Record<string, string | boolean | ((event: Event) => void)>;

export function el<K extends keyof HTMLElementTagNameMap>(tag: K, props: Props = {}, ...children: (Node | string)[]): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (typeof value === "function") node.addEventListener(key, value);
    else if (key === "class") node.className = String(value);
    else if (key === "text") node.textContent = String(value);
    else if (value !== false) node.setAttribute(key, value === true ? "" : value);
  }
  node.append(...children);
  return node;
}

/** An inline icon from a path, drawn in the current text colour. */
export function icon(path: string): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 20 20");
  svg.setAttribute("aria-hidden", "true");
  const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
  p.setAttribute("d", path);
  svg.append(p);
  return svg;
}
