import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Astra Wealth",
    short_name: "Astra",
    description: "Seu patrimônio, carteiras e mercados em um só lugar.",
    start_url: "/",
    display: "standalone",
    background_color: "#15121f",
    theme_color: "#15121f",
    icons: [{ src: "/icon.svg", sizes: "any", type: "image/svg+xml" }],
  };
}
