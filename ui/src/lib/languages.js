// What the user mostly speaks decides which voice model is best. From published error rates per
// language: FLEURS test set, on the models' quantized versions, as collected in Hex's
// docs/research/language-recommendations-european.md.
const ultra = { id: "parakeet-ultra", why: "Fewest errors at this speed, on any processor" };
const whisper = { id: "whisper-turbo", why: "Clearly fewer errors here than Parakeet. Runs best with a graphics card" };
export const languages = [
  ["de", "German", { id: "parakeet-de", why: "Trained further on German, as fast as the others" }],
  ["en", "English", { id: "parakeet-en", why: "Made for English alone, at the same speed" }],
  ["fr", "French", ultra],
  ["es", "Spanish", ultra],
  ["it", "Italian", ultra],
  ["pt", "Portuguese", ultra],
  ["nl", "Dutch", ultra],
  ["pl", "Polish", ultra],
  ["ru", "Russian", ultra],
  ["uk", "Ukrainian", ultra],
  ["cs", "Czech", ultra],
  ["ro", "Romanian", ultra],
  ["hu", "Hungarian", ultra],
  ["el", "Greek", whisper],
  ["sv", "Swedish", whisper],
  ["fi", "Finnish", whisper],
  ["da", "Danish", whisper],
  ["tr", "Turkish", whisper],
  ["ar", "Arabic", whisper],
  ["zh", "Chinese", whisper],
  ["ja", "Japanese", whisper],
  ["ko", "Korean", whisper],
  ["hi", "Hindi", whisper],
  ["other", "Another language", { id: "whisper-turbo", why: "Understands 99 languages. Runs best with a graphics card" }],
];
export const languageOptions = languages.map(([value, label]) => ({ value, label }));
export const bestFor = (code) => (languages.find(([c]) => c === code) ?? languages.at(-1))[2];

const KEY = "vorto.models.language";
const known = (code) => languages.some(([c]) => c === code);
/** The language chosen on this PC, else that of Windows. */
export function spokenLanguage() {
  let saved = null;
  try {
    saved = localStorage.getItem(KEY);
  } catch {}
  const windows = navigator.language?.slice(0, 2).toLowerCase() ?? "";
  return known(saved) ? saved : known(windows) ? windows : "other";
}
export function rememberLanguage(code) {
  try {
    localStorage.setItem(KEY, code);
  } catch {}
}
