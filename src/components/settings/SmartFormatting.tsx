import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface SmartFormattingProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const SmartFormattingToggle: React.FC<SmartFormattingProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("smart_format_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("smart_format_enabled", value)}
        isUpdating={isUpdating("smart_format_enabled")}
        label={t("settings.advanced.smartFormat.label")}
        description={t("settings.advanced.smartFormat.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });

export const SpokenCommandsToggle: React.FC<SmartFormattingProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const smartFormatEnabled = getSetting("smart_format_enabled") ?? true;
    const enabled = getSetting("spoken_commands_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("spoken_commands_enabled", value)}
        isUpdating={isUpdating("spoken_commands_enabled")}
        disabled={!smartFormatEnabled}
        label={t("settings.advanced.spokenCommands.label")}
        description={t("settings.advanced.spokenCommands.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
