import { HashRouter, Outlet, Route, Routes } from "react-router-dom";
import DashboardPage from "./pages/DashboardPage";
import OnboardingPage from "./pages/OnboardingPage";
import AddConfigurationPage from "./pages/AddConfigurationPage";
import ArchivedPage from "./pages/ArchivedPage";
import ConfigDetailPage from "./pages/ConfigDetailPage";
import DiffPage from "./pages/DiffPage";
import EditorPage from "./pages/EditorPage";
import Footer from "./components/Footer";
import SponsorBanner from "./components/SponsorBanner";

/** Wraps every page except onboarding with the persistent credit footer and
 * sponsor bar, mirroring easyalias's always-visible `.app-footer` / `.support-banner`. */
function AppShell() {
  return (
    <>
      <div className="app-main">
        <Outlet />
        <Footer />
      </div>
      <SponsorBanner />
    </>
  );
}

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/onboarding" element={<OnboardingPage />} />
        <Route element={<AppShell />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/add" element={<AddConfigurationPage />} />
          <Route path="/archived" element={<ArchivedPage />} />
          <Route path="/configurations/:id" element={<ConfigDetailPage />} />
          <Route path="/configurations/:id/edit" element={<EditorPage />} />
          <Route path="/configurations/:id/diff" element={<DiffPage mode="working" />} />
          <Route path="/configurations/:id/history/:commit" element={<DiffPage mode="snapshot" />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
