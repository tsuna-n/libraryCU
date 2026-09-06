use crate::security::redact_sensitive;

/// Total user-context budget, including headings, identifiers and history.
pub const MAX_PROMPT_CHARS: usize = 32_000;
/// Keep `--ai` focused on a small set of concrete edits instead of a second,
/// essay-style diagnostic report.
pub const MAX_CONCISE_RESPONSE_TOKENS: u32 = 512;
const TRUNCATED: &str = "\n... [truncated]";

pub(crate) fn concise_edit_instructions(language: &str) -> &'static str {
    if language == "th" {
        "ตอบเฉพาะสิ่งที่ผู้ใช้ต้องแก้ ไม่เกิน 3 จุด โดยแต่ละจุดใช้รูปแบบ `แก้: <ไฟล์:บรรทัด หรือชื่อตำแหน่ง>`, `จาก: <ข้อความหรือโค้ดเดิม>`, `เป็น: <ข้อความหรือโค้ดใหม่>` เท่านั้น ห้ามเกริ่น ห้ามทวนปัญหา ห้ามอธิบายสาเหตุ ห้ามสรุป และห้ามเสนอหลายทางเลือก ถ้าหาตำแหน่งหรือค่าที่ต้องแก้จากหลักฐานไม่ได้ ให้ตอบเพียงหนึ่งประโยคว่าขาดข้อมูลอะไรและห้ามเดา"
    } else {
        "Return only the edits the user must make, with at most 3 changes. For each change use `Change: <file:line or symbol>`, `From: <current text or code>`, and `To: <replacement text or code>`. Do not add a preface, restate the problem, explain the cause, summarize, or offer alternatives. If the evidence does not identify an exact location or replacement, return one sentence stating what evidence is missing and do not guess."
    }
}

pub(crate) struct Prompt {
    text: String,
    remaining: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            remaining: MAX_PROMPT_CHARS,
        }
    }

    pub fn push(&mut self, text: &str, budget: usize) {
        let text = bounded_redacted(text, budget.min(self.remaining));
        self.remaining -= text.chars().count();
        self.text.push_str(&text);
    }

    pub fn finish(self) -> String {
        self.text
    }
}

/// Redact first: truncation must not turn a credential into an unrecognized
/// prefix, or remove a private-key marker before it reaches the filter.
pub(crate) fn bounded_redacted(text: &str, budget: usize) -> String {
    let redacted = redact_sensitive(text);
    if redacted.chars().count() <= budget {
        return redacted;
    }
    let suffix_len = TRUNCATED.chars().count();
    if budget < suffix_len {
        return TRUNCATED.chars().take(budget).collect();
    }
    let mut result: String = redacted.chars().take(budget - suffix_len).collect();
    result.push_str(TRUNCATED);
    result
}
