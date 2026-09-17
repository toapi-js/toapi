// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightThemeVintage from "starlight-theme-vintage";

/** Build a package section: Introduction + optional Guides + Reference. */
function pkg(name, { guides = false } = {}) {
  const items = [{ label: "Introduction", slug: name }];
  if (guides) {
    items.push({ label: "Guides", items: [{ autogenerate: { directory: `${name}/guides` } }] });
  }
  items.push({ label: "Reference", items: [{ autogenerate: { directory: `${name}/reference` } }] });
  return { label: `@toapi/${name}`, collapsed: true, items };
}

// https://astro.build/config
export default defineConfig({
  site: "https://farbenmeer.github.io",
  base: "/tapi",
  integrations: [
    starlight({
      title: "Toapi",
      logo: { src: "./src/assets/logo.svg", alt: "Toapi" },
      favicon: "/favicon.svg",
      head: [
        // Starlight's `favicon` is base-prefixed automatically; these custom
        // links are not, so include the /tapi base explicitly.
        { tag: "link", attrs: { rel: "icon", type: "image/png", sizes: "96x96", href: "/tapi/favicon-96x96.png" } },
        { tag: "link", attrs: { rel: "shortcut icon", href: "/tapi/favicon.ico" } },
        { tag: "link", attrs: { rel: "apple-touch-icon", sizes: "180x180", href: "/tapi/apple-touch-icon.png" } },
        { tag: "link", attrs: { rel: "manifest", href: "/tapi/site.webmanifest" } },
      ],
      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/farbenmeer/tapi" },
      ],
      plugins: [starlightThemeVintage()],
      sidebar: [
        pkg("server", { guides: true }),
        pkg("client"),
        pkg("worker", { guides: true }),
        pkg("common"),
        pkg("cache"),
        pkg("router", { guides: true }),
        pkg("react"),
        pkg("vite-plugin", { guides: true }),
      ],
    }),
  ],
});
