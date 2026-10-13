// Derleme çıktılarını release/ klasörüne toplar:
//   Arunnel.exe              → kurulumsuz (taşınabilir) sürüm
//   Arunnel_x.y.z_Setup.exe  → kurulum sihirbazı
// Depo ile yan yana bir ../exe klasörü varsa Arunnel.exe oraya da kopyalanır.
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";

const { version } = JSON.parse(readFileSync("package.json", "utf8"));
const target = "src-tauri/target/release";
mkdirSync("release", { recursive: true });

copyFileSync(`${target}/arunnel.exe`, "release/Arunnel.exe");
const setup = readdirSync(`${target}/bundle/nsis`).find((f) => f.endsWith("-setup.exe"));
if (setup) copyFileSync(`${target}/bundle/nsis/${setup}`, `release/Arunnel_${version}_Setup.exe`);
if (existsSync("../exe")) copyFileSync(`${target}/arunnel.exe`, "../exe/Arunnel.exe");
console.log("release/ klasörü hazır");
