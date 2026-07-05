import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface TextReplacementsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const TextReplacements: React.FC<TextReplacementsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [pattern, setPattern] = useState("");
    const [replacement, setReplacement] = useState("");
    const replacements = getSetting("text_replacements") || [];

    const canAdd =
      pattern.trim().length > 0 &&
      pattern.trim().length <= 60 &&
      replacement.trim().length > 0 &&
      !replacements.some(
        (r) => r.pattern.toLowerCase() === pattern.trim().toLowerCase(),
      );

    const handleAdd = () => {
      if (!canAdd) return;
      updateSetting("text_replacements", [
        ...replacements,
        {
          pattern: pattern.trim(),
          replacement: replacement.trim(),
          case_sensitive: false,
        },
      ]);
      setPattern("");
      setReplacement("");
    };

    const handleRemove = (patternToRemove: string) => {
      updateSetting(
        "text_replacements",
        replacements.filter((r) => r.pattern !== patternToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.textReplacements.title")}
          description={t("settings.advanced.textReplacements.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-36"
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t(
                "settings.advanced.textReplacements.patternPlaceholder",
              )}
              variant="compact"
              disabled={isUpdating("text_replacements")}
            />
            <span className="text-mid-gray">→</span>
            <Input
              type="text"
              className="max-w-36"
              value={replacement}
              onChange={(e) => setReplacement(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t(
                "settings.advanced.textReplacements.replacementPlaceholder",
              )}
              variant="compact"
              disabled={isUpdating("text_replacements")}
            />
            <Button
              onClick={handleAdd}
              disabled={!canAdd || isUpdating("text_replacements")}
              variant="primary"
              size="md"
            >
              {t("settings.advanced.textReplacements.add")}
            </Button>
          </div>
        </SettingContainer>
        {replacements.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {replacements.map((r) => (
              <Button
                key={r.pattern}
                onClick={() => handleRemove(r.pattern)}
                disabled={isUpdating("text_replacements")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.textReplacements.remove", {
                  pattern: r.pattern,
                })}
              >
                <span>
                  {r.pattern} → {r.replacement}
                </span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </>
    );
  },
);
