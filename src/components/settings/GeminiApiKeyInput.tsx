import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff } from "lucide-react";
import { useSettings } from "@/hooks/useSettings";
import { SettingContainer } from "@/components/ui/SettingContainer";
import { Input } from "@/components/ui/Input";

interface GeminiApiKeyInputProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const GeminiApiKeyInput: React.FC<GeminiApiKeyInputProps> = React.memo(
  ({ descriptionMode = "inline", grouped = false }) => {
    const { t } = useTranslation();
    const { settings, updateSetting } = useSettings();
    const [showKey, setShowKey] = useState(false);
    const apiKey = settings?.gemini_stt_api_key || "";

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      updateSetting("gemini_stt_api_key", e.target.value);
    };

    return (
      <SettingContainer
        title={t("settings.models.geminiApiKey.title")}
        description={t("settings.models.geminiApiKey.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="stacked"
      >
        <div className="relative mt-1">
          <Input
            type={showKey ? "text" : "password"}
            value={apiKey}
            onChange={handleChange}
            placeholder={t("settings.models.geminiApiKey.placeholder")}
            className="w-full font-mono pr-9"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-text/40 hover:text-text focus:outline-none"
          >
            {showKey ? (
              <EyeOff className="h-4 w-4" />
            ) : (
              <Eye className="h-4 w-4" />
            )}
          </button>
        </div>
      </SettingContainer>
    );
  },
);

GeminiApiKeyInput.displayName = "GeminiApiKeyInput";
