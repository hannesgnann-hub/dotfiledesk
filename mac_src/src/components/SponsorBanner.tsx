import { openUrl } from "@tauri-apps/plugin-opener";

const SPONSOR_URL = "https://github.com/sponsors/hannesgnann-hub";

/** Persistent bottom bar, visible on every page — mirrors easyalias's `.support-banner`. */
export default function SponsorBanner() {
  return (
    <aside className="support-banner" aria-label="Support DotfileDesk">
      <span>Support DotfileDesk development</span>
      <button className="support-banner-link" onClick={() => openUrl(SPONSOR_URL)}>
        Become a sponsor
      </button>
    </aside>
  );
}
