import { useEffect, useState } from "react";
import { getAppInfo, getSettings, setSettings, type SettingsDto } from "@/shared/ipc";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-2xl border border-edge-soft bg-panel p-5">
      <h2 className="text-[11px] font-semibold tracking-[0.16em] text-ink-faint uppercase">
        {title}
      </h2>
      <div className="mt-4">{children}</div>
    </section>
  );
}

export default function SettingsView() {
  const [settings, setState] = useState<SettingsDto | null>(null);
  const [keyInput, setKeyInput] = useState("");
  const [version, setVersion] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    void getSettings().then(setState).catch(() => undefined);
    void getAppInfo().then((i) => setVersion(i.version)).catch(() => undefined);
  }, []);

  const saveKey = async () => {
    const next = await setSettings({ geminiKey: keyInput });
    setState(next);
    setKeyInput("");
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const clearKey = async () => {
    setState(await setSettings({ geminiKey: "" }));
  };

  const setLanguage = async (lang: string) => {
    setState(await setSettings({ aiLanguage: lang }));
  };

  return (
    <div className="mx-auto max-w-2xl space-y-4 px-8 py-10">
      <h1 className="text-2xl font-bold tracking-tight">Settings</h1>

      <Section title="Gemini API key">
        <p className="text-[12px] leading-relaxed text-ink-mute">
          Powers AI Insights. Get a free key at{" "}
          <span className="font-mono text-ink">aistudio.google.com</span>. The key is
          stored locally and used only for requests you trigger.
        </p>
        <div className="mt-3 flex items-center gap-2">
          <input
            type="password"
            value={keyInput}
            onChange={(e) => setKeyInput(e.target.value)}
            placeholder={
              settings?.hasGeminiKey
                ? settings.keySource === "env"
                  ? "Using key from environment (.env)"
                  : "Key configured — paste to replace"
                : "Paste your API key"
            }
            className="min-w-0 flex-1 rounded-xl border border-edge bg-void px-3.5 py-2 font-mono text-[12px] text-ink placeholder:text-ink-faint focus:border-glow-cyan focus:outline-none"
          />
          <button
            onClick={() => void saveKey()}
            disabled={keyInput.trim().length === 0}
            className="rounded-xl bg-gradient-glow px-4 py-2 text-[13px] font-semibold text-void disabled:opacity-40"
          >
            {saved ? "Saved ✓" : "Save"}
          </button>
        </div>
        <div className="mt-2.5 flex items-center gap-3">
          <span
            className={`h-2 w-2 rounded-full ${settings?.hasGeminiKey ? "bg-ok" : "bg-danger"}`}
          />
          <span className="text-[12px] text-ink-faint">
            {settings?.hasGeminiKey
              ? `Key active (source: ${settings.keySource})`
              : "No key configured — AI Insights disabled"}
          </span>
          {settings?.keySource === "settings" && (
            <button
              onClick={() => void clearKey()}
              className="text-[12px] text-ink-faint underline-offset-2 hover:text-danger hover:underline"
            >
              Remove
            </button>
          )}
        </div>
      </Section>

      <Section title="AI response language">
        <div className="flex gap-1.5">
          {[
            { id: "en", label: "English" },
            { id: "ru", label: "Русский" },
          ].map((l) => (
            <button
              key={l.id}
              onClick={() => void setLanguage(l.id)}
              className={`rounded-lg border px-4 py-1.5 text-[13px] font-semibold transition-colors ${
                settings?.aiLanguage === l.id
                  ? "border-transparent bg-overlay text-ink"
                  : "border-edge text-ink-faint hover:text-ink-mute"
              }`}
            >
              {l.label}
            </button>
          ))}
        </div>
      </Section>

      <Section title="Privacy">
        <ul className="space-y-2 text-[12px] leading-relaxed text-ink-mute">
          <li>· AI receives only an anonymized statistical digest (see Data passport).</li>
          <li>· File names and contents never leave this device.</li>
          <li>· Deletions always go to the Recycle Bin first.</li>
          <li>· Hash cache and settings live in %LOCALAPPDATA%\com.nkvas1.diskovery.</li>
        </ul>
      </Section>

      <p className="pt-2 text-center font-mono text-[11px] tracking-widest text-ink-faint">
        DISKOVERY v{version} · MIT · github.com/nkVas1/diskovery
      </p>
    </div>
  );
}
