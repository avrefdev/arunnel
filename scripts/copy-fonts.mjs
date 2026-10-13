// Inter fontunu node_modules'tan ui/fonts klasörüne kopyalar.
// Font uygulamanın içine gömülür; internet olmasa da arayüz aynı görünür.
import { copyFileSync, mkdirSync } from "node:fs";

const src = "node_modules/@fontsource-variable/inter/files";
mkdirSync("ui/fonts", { recursive: true });
for (const subset of ["latin", "latin-ext"]) {
  copyFileSync(`${src}/inter-${subset}-wght-normal.woff2`, `ui/fonts/inter-${subset}.woff2`);
}
console.log("fontlar kopyalandı");
