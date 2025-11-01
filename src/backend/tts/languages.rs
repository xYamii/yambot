use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub code: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub languages: HashMap<String, Language>,
}

impl LanguageConfig {
    pub fn new() -> Self {
        let mut languages = HashMap::new();

        // Initialize all languages from requirements
        let language_list = vec![
            ("af", "Afrikaans"),
            ("sq", "Albanian"),
            ("am", "Amharic"),
            ("ar", "Arabic"),
            ("az", "Azerbaijani"),
            ("be", "Belarusian"),
            ("bg", "Bulgarian"),
            ("bn", "Bengali"),
            ("bs", "Bosnian"),
            ("ca", "Catalan"),
            ("ceb", "Cebuano"),
            ("ny", "Chichewa"),
            ("zh-CN", "Chinese (Simplified)"),
            ("zh-TW", "Chinese (Traditional)"),
            ("co", "Corsican"),
            ("hr", "Croatian"),
            ("cs", "Czech"),
            ("da", "Danish"),
            ("nl", "Dutch"),
            ("en", "English"),
            ("eo", "Esperanto"),
            ("et", "Estonian"),
            ("tl", "Filipino"),
            ("fi", "Finnish"),
            ("fr", "French"),
            ("fy", "Frisian"),
            ("gl", "Galician"),
            ("ka", "Georgian"),
            ("de", "German"),
            ("el", "Greek"),
            ("gu", "Gujarati"),
            ("ht", "Haitian Creole"),
            ("ha", "Hausa"),
            ("haw", "Hawaiian"),
            ("iw", "Hebrew"),
            ("hi", "Hindi"),
            ("hmn", "Hmong"),
            ("hu", "Hungarian"),
            ("is", "Icelandic"),
            ("ig", "Igbo"),
            ("id", "Indonesian"),
            ("ga", "Irish"),
            ("it", "Italian"),
            ("ja", "Japanese"),
            ("jw", "Javanese"),
            ("kn", "Kannada"),
            ("kk", "Kazakh"),
            ("km", "Khmer"),
            ("rw", "Kinyarwanda"),
            ("ko", "Korean"),
            ("ku", "Kurdish (Kurmanji)"),
            ("ky", "Kyrgyz"),
            ("lo", "Lao"),
            ("la", "Latin"),
            ("lv", "Latvian"),
            ("lt", "Lithuanian"),
            ("lb", "Luxembourgish"),
            ("mk", "Macedonian"),
            ("mg", "Malagasy"),
            ("ms", "Malay"),
            ("ml", "Malayalam"),
            ("mt", "Maltese"),
            ("mi", "Maori"),
            ("mr", "Marathi"),
            ("mn", "Mongolian"),
            ("my", "Myanmar (Burmese)"),
            ("ne", "Nepali"),
            ("no", "Norwegian"),
            ("or", "Odia"),
            ("ps", "Pashto"),
            ("fa", "Persian"),
            ("pl", "Polish"),
            ("pt", "Portuguese"),
            ("pa", "Punjabi"),
            ("qu", "Quechua"),
            ("ro", "Romanian"),
            ("ru", "Russian"),
            ("sm", "Samoan"),
            ("gd", "Scots Gaelic"),
            ("sr", "Serbian"),
            ("st", "Sesotho"),
            ("sn", "Shona"),
            ("sd", "Sindhi"),
            ("si", "Sinhala"),
            ("sk", "Slovak"),
            ("sl", "Slovenian"),
            ("so", "Somali"),
            ("es", "Spanish"),
            ("su", "Sundanese"),
            ("sw", "Swahili"),
            ("sv", "Swedish"),
            ("tg", "Tajik"),
            ("ta", "Tamil"),
            ("te", "Telugu"),
            ("th", "Thai"),
            ("tr", "Turkish"),
            ("uk", "Ukrainian"),
            ("ur", "Urdu"),
            ("ug", "Uyghur"),
            ("uz", "Uzbek"),
            ("vi", "Vietnamese"),
            ("cy", "Welsh"),
            ("xh", "Xhosa"),
            ("yi", "Yiddish"),
            ("yo", "Yoruba"),
            ("zu", "Zulu"),
        ];

        for (code, name) in language_list {
            languages.insert(
                code.to_string(),
                Language {
                    code: code.to_string(),
                    name: name.to_string(),
                    enabled: true, // All enabled by default as per requirements
                },
            );
        }

        Self { languages }
    }

    pub fn get_language(&self, code: &str) -> Option<&Language> {
        self.languages.get(code)
    }

    pub fn is_enabled(&self, code: &str) -> bool {
        self.languages
            .get(code)
            .map(|lang| lang.enabled)
            .unwrap_or(false)
    }

    pub fn toggle_language(&mut self, code: &str) {
        if let Some(lang) = self.languages.get_mut(code) {
            lang.enabled = !lang.enabled;
        }
    }

    pub fn enable_language(&mut self, code: &str) {
        if let Some(lang) = self.languages.get_mut(code) {
            lang.enabled = true;
        }
    }

    pub fn disable_language(&mut self, code: &str) {
        if let Some(lang) = self.languages.get_mut(code) {
            lang.enabled = false;
        }
    }

    pub fn get_enabled_languages(&self) -> Vec<&Language> {
        self.languages
            .values()
            .filter(|lang| lang.enabled)
            .collect()
    }

    pub fn get_all_languages(&self) -> Vec<&Language> {
        let mut langs: Vec<&Language> = self.languages.values().collect();
        langs.sort_by(|a, b| a.name.cmp(&b.name));
        langs
    }
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self::new()
    }
}
