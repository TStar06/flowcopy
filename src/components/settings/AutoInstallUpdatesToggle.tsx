import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface AutoInstallUpdatesToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const AutoInstallUpdatesToggle: React.FC<
  AutoInstallUpdatesToggleProps
> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const autoInstall = getSetting("auto_install_updates") ?? true;
  const updateChecksEnabled = getSetting("update_checks_enabled") ?? true;

  return (
    <ToggleSwitch
      checked={autoInstall}
      onChange={(enabled) => updateSetting("auto_install_updates", enabled)}
      isUpdating={isUpdating("auto_install_updates")}
      label={t("settings.debug.autoInstallUpdates.label")}
      description={t("settings.debug.autoInstallUpdates.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={!updateChecksEnabled}
    />
  );
};
