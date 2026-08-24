import { HashRouter, Route, Routes } from "react-router-dom";
import DashboardPage from "./pages/DashboardPage";
import OnboardingPage from "./pages/OnboardingPage";
import AddConfigurationPage from "./pages/AddConfigurationPage";
import ConfigDetailPage from "./pages/ConfigDetailPage";
import HistoryPage from "./pages/HistoryPage";
import DiffPage from "./pages/DiffPage";
import EditorPage from "./pages/EditorPage";

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/onboarding" element={<OnboardingPage />} />
        <Route path="/add" element={<AddConfigurationPage />} />
        <Route path="/configurations/:id" element={<ConfigDetailPage />} />
        <Route path="/configurations/:id/edit" element={<EditorPage />} />
        <Route path="/configurations/:id/diff" element={<DiffPage mode="working" />} />
        <Route path="/configurations/:id/history" element={<HistoryPage />} />
        <Route path="/configurations/:id/history/:commit" element={<DiffPage mode="snapshot" />} />
      </Routes>
    </HashRouter>
  );
}
