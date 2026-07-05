import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface AppProfilesProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

const selectClasses =
  "px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/80 rounded-md focus:outline-none focus:border-logo-primary";

export const AppProfiles: React.FC<AppProfilesProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [exeMatch, setExeMatch] = useState("");
    const [promptId, setPromptId] = useState("");
    const [cleanup, setCleanup] = useState("default");
    const profiles = getSetting("app_profiles") || [];
    const prompts = getSetting("post_process_prompts") || [];

    const canAdd =
      exeMatch.trim().length > 0 &&
      !profiles.some(
        (p) => p.exe_match.toLowerCase() === exeMatch.trim().toLowerCase(),
      );

    const handleAdd = () => {
      if (!canAdd) return;
      updateSetting("app_profiles", [
        ...profiles,
        {
          exe_match: exeMatch.trim().toLowerCase(),
          prompt_id: promptId || null,
          post_process:
            cleanup === "default" ? null : cleanup === "on" ? true : false,
        },
      ]);
      setExeMatch("");
      setPromptId("");
      setCleanup("default");
    };

    const handleRemove = (match: string) => {
      updateSetting(
        "app_profiles",
        profiles.filter((p) => p.exe_match !== match),
      );
    };

    const promptName = (id: string | null | undefined) =>
      prompts.find((p) => p.id === id)?.name ??
      t("settings.advanced.appProfiles.defaultPrompt");

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.appProfiles.title")}
          description={t("settings.advanced.appProfiles.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          layout="stacked"
        >
          <div className="flex flex-wrap items-center gap-2 w-full">
            <Input
              type="text"
              className="max-w-36"
              value={exeMatch}
              onChange={(e) => setExeMatch(e.target.value)}
              placeholder={t("settings.advanced.appProfiles.exePlaceholder")}
              variant="compact"
              disabled={isUpdating("app_profiles")}
            />
            <select
              className={selectClasses}
              value={promptId}
              onChange={(e) => setPromptId(e.target.value)}
              disabled={isUpdating("app_profiles")}
            >
              <option value="">
                {t("settings.advanced.appProfiles.defaultPrompt")}
              </option>
              {prompts.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <select
              className={selectClasses}
              value={cleanup}
              onChange={(e) => setCleanup(e.target.value)}
              disabled={isUpdating("app_profiles")}
            >
              <option value="default">
                {t("settings.advanced.appProfiles.cleanupDefault")}
              </option>
              <option value="on">
                {t("settings.advanced.appProfiles.cleanupOn")}
              </option>
              <option value="off">
                {t("settings.advanced.appProfiles.cleanupOff")}
              </option>
            </select>
            <Button
              onClick={handleAdd}
              disabled={!canAdd || isUpdating("app_profiles")}
              variant="primary"
              size="md"
            >
              {t("settings.advanced.appProfiles.add")}
            </Button>
          </div>
        </SettingContainer>
        {profiles.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {profiles.map((p) => (
              <Button
                key={p.exe_match}
                onClick={() => handleRemove(p.exe_match)}
                disabled={isUpdating("app_profiles")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.appProfiles.remove", {
                  exe: p.exe_match,
                })}
              >
                <span>
                  {p.exe_match} → {promptName(p.prompt_id)}
                  {p.post_process === false &&
                    ` (${t("settings.advanced.appProfiles.cleanupOff")})`}
                  {p.post_process === true &&
                    ` (${t("settings.advanced.appProfiles.cleanupOn")})`}
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
