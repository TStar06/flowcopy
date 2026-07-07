import React from "react";
import { useTranslation } from "react-i18next";
import { ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Button } from "../../ui/Button";

/// The one place a non-technical user sets up the free Groq key. Explains what
/// it does, links straight to the key page, and exposes the two switches
/// (cleanup + cloud transcription) without hunting through Advanced settings.
export const GroqQuickstart: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const postProcessEnabled = getSetting("post_process_enabled") ?? false;
  const cloudEnabled = getSetting("cloud_transcription_enabled") ?? false;

  return (
    <div className="rounded-lg border border-logo-primary/40 bg-logo-primary/5 p-4 space-y-3">
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">
          {t("settings.postProcessing.quickstart.title")}
        </h3>
        <p className="text-sm text-text/70">
          {t("settings.postProcessing.quickstart.body")}
        </p>
      </div>
      <Button
        variant="primary"
        size="md"
        onClick={() => openUrl("https://console.groq.com/keys")}
        className="inline-flex items-center gap-2"
      >
        {t("settings.postProcessing.quickstart.getKey")}
        <ExternalLink className="w-4 h-4" />
      </Button>
      <div className="pt-1 space-y-2">
        <ToggleSwitch
          checked={cloudEnabled}
          onChange={(v) => updateSetting("cloud_transcription_enabled", v)}
          isUpdating={isUpdating("cloud_transcription_enabled")}
          label={t("settings.postProcessing.quickstart.cloudLabel")}
          description={t("settings.postProcessing.quickstart.cloudDescription")}
          descriptionMode="inline"
          grouped
        />
        <ToggleSwitch
          checked={postProcessEnabled}
          onChange={(v) => updateSetting("post_process_enabled", v)}
          isUpdating={isUpdating("post_process_enabled")}
          label={t("settings.postProcessing.quickstart.cleanupLabel")}
          description={t(
            "settings.postProcessing.quickstart.cleanupDescription",
          )}
          descriptionMode="inline"
          grouped
        />
      </div>
    </div>
  );
});

GroqQuickstart.displayName = "GroqQuickstart";
