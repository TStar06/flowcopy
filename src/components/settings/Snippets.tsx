import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Textarea } from "../ui/Textarea";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface SnippetsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const Snippets: React.FC<SnippetsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [trigger, setTrigger] = useState("");
    const [body, setBody] = useState("");
    const snippets = getSetting("snippets") || [];

    const canAdd =
      trigger.trim().length > 0 &&
      trigger.trim().length <= 60 &&
      body.trim().length > 0 &&
      body.length <= 4000 &&
      !snippets.some(
        (s) => s.trigger.toLowerCase() === trigger.trim().toLowerCase(),
      );

    const handleAdd = () => {
      if (!canAdd) return;
      updateSetting("snippets", [
        ...snippets,
        { trigger: trigger.trim(), body: body },
      ]);
      setTrigger("");
      setBody("");
    };

    const handleRemove = (triggerToRemove: string) => {
      updateSetting(
        "snippets",
        snippets.filter((s) => s.trigger !== triggerToRemove),
      );
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.snippets.title")}
          description={t("settings.advanced.snippets.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          layout="stacked"
        >
          <div className="flex flex-col gap-2 w-full">
            <Input
              type="text"
              value={trigger}
              onChange={(e) => setTrigger(e.target.value)}
              placeholder={t("settings.advanced.snippets.triggerPlaceholder")}
              variant="compact"
              disabled={isUpdating("snippets")}
            />
            <Textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder={t("settings.advanced.snippets.bodyPlaceholder")}
              rows={3}
              disabled={isUpdating("snippets")}
            />
            <div>
              <Button
                onClick={handleAdd}
                disabled={!canAdd || isUpdating("snippets")}
                variant="primary"
                size="md"
              >
                {t("settings.advanced.snippets.add")}
              </Button>
            </div>
          </div>
        </SettingContainer>
        {snippets.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {snippets.map((s) => (
              <Button
                key={s.trigger}
                onClick={() => handleRemove(s.trigger)}
                disabled={isUpdating("snippets")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.snippets.remove", {
                  trigger: s.trigger,
                })}
                title={s.body}
              >
                <span>{s.trigger}</span>
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
