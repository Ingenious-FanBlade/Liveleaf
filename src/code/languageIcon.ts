import { escapeCode } from "./syntaxHighlight";
import pythonIcon from "../assets/icons/python.svg?raw";

/** Inline icons keep preview headers available offline and in packaged lessons. */
export function renderLanguageIcon(language: string): string {
  const name = language.toLowerCase();
  let artwork: string;
  let label: string;

  switch (name) {
    case "python": case "python3": case "py":
      return pythonIcon;
    case "javascript": case "js": case "mjs": case "cjs":
      label = "JavaScript";
      artwork = '<rect x="2" y="2" width="20" height="20" rx="2" fill="#f7df1e"/><path fill="#263747" d="M11 9h2v8c0 2-1 3-3 3-1.5 0-2.5-.7-3-2l1.7-1c.3.7.6 1 1.3 1s1-.4 1-1V9Zm8.6 2.8c-.5-.7-1-.9-1.6-.9-.7 0-1 .3-1 .8 0 .6.5.9 1.5 1.3 2 .8 3 1.7 3 3.3 0 2-1.5 3.2-3.5 3.2-1.8 0-3-.9-3.6-2.2l1.8-1c.4.8 1 1.3 1.8 1.3.9 0 1.4-.4 1.4-1 0-.7-.6-1-1.8-1.5-1.8-.8-2.7-1.6-2.7-3.2 0-1.7 1.3-2.9 3.1-2.9 1.4 0 2.4.6 3 1.7l-1.4 1.1Z"/>';
      break;
    case "typescript": case "ts": case "tsx":
      label = "TypeScript";
      artwork = '<rect x="2" y="2" width="20" height="20" rx="2" fill="#3178c6"/><path fill="#fff" d="M4 10h9v2H9.5v8h-2v-8H4v-2Zm15.6 2.2c-.5-.7-1-.9-1.6-.9-.7 0-1 .3-1 .8 0 .6.5.9 1.5 1.3 2 .8 3 1.7 3 3.3 0 2-1.5 3.2-3.5 3.2-1.8 0-3-.9-3.6-2.2l1.8-1c.4.8 1 1.3 1.8 1.3.9 0 1.4-.4 1.4-1 0-.7-.6-1-1.8-1.5-1.8-.8-2.7-1.6-2.7-3.2 0-1.7 1.3-2.9 3.1-2.9 1.4 0 2.4.6 3 1.7l-1.4 1.1Z"/>';
      break;
    case "jsx":
      label = "React JSX";
      artwork = '<g fill="none" stroke="currentColor" stroke-width="1.4"><ellipse cx="12" cy="12" rx="10" ry="4"/><ellipse cx="12" cy="12" rx="10" ry="4" transform="rotate(60 12 12)"/><ellipse cx="12" cy="12" rx="10" ry="4" transform="rotate(120 12 12)"/></g><circle cx="12" cy="12" r="2" fill="currentColor"/>';
      break;
    case "json":
      label = "JSON";
      artwork = '<path d="M9 3H7a2 2 0 0 0-2 2v4c0 2-2 3-2 3s2 1 2 3v4a2 2 0 0 0 2 2h2m6-18h2a2 2 0 0 1 2 2v4c0 2 2 3 2 3s-2 1-2 3v4a2 2 0 0 1-2 2h-2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>';
      break;
    default:
      label = language;
      artwork = '<path d="m8 6-6 6 6 6m8-12 6 6-6 6m-3-14-2 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>';
  }

  const escapedLabel = escapeCode(label);
  return `<svg class="sl-code-cell__language-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="22" height="22" role="img" aria-label="${escapedLabel}"><title>${escapedLabel}</title>${artwork}</svg>`;
}
