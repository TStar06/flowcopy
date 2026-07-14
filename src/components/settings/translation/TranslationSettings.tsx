import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Dropdown } from "../../ui/Dropdown";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";
import { TRANSLATION_TARGET_LANGUAGES } from "../../../lib/constants/languages";
import { commands } from "@/bindings";

const TRANSLATE_BINDING_PREFIX = "transcribe_translate:";

export const TranslationSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const [isMutating, setIsMutating] = useState(false);

  const enabled = getSetting("translate_enabled") ?? false;
  const bindings = getSetting("bindings") || {};

  const translateBindings = useMemo(
    () =>
      Object.values(bindings)
        .filter(
          (b): b is NonNullable<typeof b> =>
            !!b && b.id.startsWith(TRANSLATE_BINDING_PREFIX),
        )
        .sort((a, b) => a.id.localeCompare(b.id)),
    [bindings],
  );

  const usedLanguages = useMemo(
    () =>
      new Set(
        translateBindings.map((b) => b.id.slice(TRANSLATE_BINDING_PREFIX.length)),
      ),
    [translateBindings],
  );
  const availableLanguages = TRANSLATION_TARGET_LANGUAGES.filter(
    (lang) => !usedLanguages.has(lang.value),
  );
  const [newLanguage, setNewLanguage] = useState<string>("");

  const addTarget = async () => {
    const language = newLanguage || availableLanguages[0]?.value;
    if (!language) return;
    setIsMutating(true);
    try {
      const result = await commands.addTranslateTarget(language);
      if (result.status === "error") {
        toast.error(String(result.error));
      }
      await refreshSettings();
      setNewLanguage("");
    } finally {
      setIsMutating(false);
    }
  };

  const removeTarget = async (id: string) => {
    setIsMutating(true);
    try {
      const result = await commands.removeTranslateTarget(id);
      if (result.status === "error") {
        toast.error(String(result.error));
      }
      await refreshSettings();
    } finally {
      setIsMutating(false);
    }
  };

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
        {translateBindings.map((binding) => (
          <div key={binding.id} className="flex items-center gap-2">
            <div className="flex-1 min-w-0">
              <ShortcutInput
                shortcutId={binding.id}
                grouped={true}
                disabled={!enabled}
              />
            </div>
            <button
              className="p-2 mr-2 rounded-md text-text/50 hover:text-red-400 hover:bg-red-400/10 transition-colors shrink-0"
              onClick={() => removeTarget(binding.id)}
              disabled={isMutating}
              title={t("settings.translation.remove")}
            >
              <Trash2 size={16} />
            </button>
          </div>
        ))}
        <SettingContainer
          title={t("settings.translation.addLanguage.label")}
          description={t("settings.translation.addLanguage.description")}
          descriptionMode="inline"
          grouped={true}
        >
          <div className="flex items-center gap-2">
            <Dropdown
              options={availableLanguages.map((lang) => ({
                value: lang.value,
                label: lang.label,
              }))}
              selectedValue={newLanguage || availableLanguages[0]?.value || ""}
              onSelect={(value) => setNewLanguage(value)}
              disabled={isMutating || availableLanguages.length === 0}
            />
            <button
              className="flex items-center gap-1 px-3 py-1.5 text-sm rounded-md bg-logo-primary/20 hover:bg-logo-primary/30 transition-colors disabled:opacity-50"
              onClick={addTarget}
              disabled={isMutating || availableLanguages.length === 0}
            >
              <Plus size={14} />
              {t("settings.translation.addLanguage.button")}
            </button>
          </div>
        </SettingContainer>
      </SettingsGroup>
      <p className="text-sm text-text/60 px-1">
        {t("settings.translation.hint")}
      </p>
    </div>
  );
};
