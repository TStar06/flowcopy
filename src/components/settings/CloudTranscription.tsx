import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface CloudTranscriptionProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const CloudTranscription: React.FC<CloudTranscriptionProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("cloud_transcription_enabled") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) =>
          updateSetting("cloud_transcription_enabled", value)
        }
        isUpdating={isUpdating("cloud_transcription_enabled")}
        label={t("settings.general.cloudTranscription.label")}
        description={t("settings.general.cloudTranscription.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
