import React from "react";
import { useTranslation } from "react-i18next";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Dropdown } from "../../ui/Dropdown";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";
import { TRANSLATION_TARGET_LANGUAGES } from "../../../lib/constants/languages";

export const TranslationSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const enabled = getSetting("translate_enabled") ?? false;
  const targetLanguage = getSetting("translate_target_language") || "pl";

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.translation.title")}>
        <ToggleSwitch
          checked={enabled}
          onChange={(value) => updateSetting("translate_enabled", value)}
          isUpdating={isUpdating("translate_enabled")}
          label={t("settings.translation.enable.label")}
          description={t("settings.translation.enable.description")}
          descriptionMode="inline"
          grouped={true}
        />
        <ShortcutInput
          shortcutId="transcribe_translate"
          grouped={true}
          disabled={!enabled}
        />
        <SettingContainer
          title={t("settings.translation.targetLanguage.label")}
          description={t("settings.translation.targetLanguage.description")}
          descriptionMode="inline"
          grouped={true}
        >
          <Dropdown
            options={TRANSLATION_TARGET_LANGUAGES.map((lang) => ({
              value: lang.value,
              label: lang.label,
            }))}
            selectedValue={targetLanguage}
            onSelect={(value) =>
              updateSetting("translate_target_language", value)
            }
            disabled={!enabled || isUpdating("translate_target_language")}
          />
        </SettingContainer>
      </SettingsGroup>
      <p className="text-sm text-text/60 px-1">
        {t("settings.translation.hint")}
      </p>
    </div>
  );
};
