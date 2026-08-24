import { openUrl } from "@tauri-apps/plugin-opener";

const REPO_URL = "https://github.com/hannesgnann-hub/dotfiledesk";
const ISSUES_URL = "https://github.com/hannesgnann-hub/dotfiledesk/issues";

/** Inline credit line under every page's content — mirrors easyalias's `.app-footer`. */
export default function Footer() {
  const year = new Date().getFullYear();
  return (
    <footer className="app-footer">
      <button className="app-footer-link" onClick={() => openUrl(REPO_URL)}>
        © {year} Hannes Gnann
      </button>
      <span aria-hidden="true">·</span>
      <button className="app-footer-link" onClick={() => openUrl(ISSUES_URL)}>
        Report an Issue
      </button>
    </footer>
  );
}
