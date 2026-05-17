pub const DEFAULT_VOICE: &str = "zh-CN-XiaoxiaoNeural";
pub const MAX_CHUNK_SIZE: usize = 4000;
pub const SUPPORTED_FORMATS: &[&str] = &[".md", ".txt", ".html", ".htm"];

pub const VOICE_ALIASES: &[(&str, &str)] = &[
    ("xiaoxiao", "zh-CN-XiaoxiaoNeural"),
    ("yunxi", "zh-CN-YunxiNeural"),
    ("xiaohan", "zh-CN-XiaohanNeural"),
    ("yunyang", "zh-CN-YunyangNeural"),
];

pub fn alias_to_voice(alias: &str) -> Option<&'static str> {
    VOICE_ALIASES
        .iter()
        .find(|(k, _)| *k == alias)
        .map(|(_, v)| *v)
}

pub fn voice_to_alias(voice: &str) -> Option<&'static str> {
    VOICE_ALIASES
        .iter()
        .find(|(_, v)| *v == voice)
        .map(|(k, _)| *k)
}
