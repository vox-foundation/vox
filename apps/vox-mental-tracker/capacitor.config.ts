import type { CapacitorConfig } from "@capacitor/cli";

const config: CapacitorConfig = {
  appId: "com.vox.mentaltracker",
  appName: "Vox Mental Tracker",
  webDir: "web-dist",
  server: {
    androidScheme: "https",
  },
};

export default config;
