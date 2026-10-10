import type { AgentLook } from "./cases";
import s from "./scene.module.css";

export interface Line {
  who: "you" | "did" | "says";
  text: string;
}

/// The two agents' marks: what each puts before a prompt and before its own lines. The windows
/// are drawn in each agent's manner so they are recognised; neither is a copy of the real one.
const LOOKS: Record<AgentLook, { name: string; prompt: string; bullet: string }> = {
  claude: { name: "Claude Code", prompt: ">", bullet: "●" },
  codex: { name: "Codex", prompt: "›", bullet: "•" },
};

/// The agent's terminal, beside the thing being built. `typed` is what is in its prompt.
export default function Terminal({ look, folder, lines, typed, working, promptRef }: { look: AgentLook; folder: string; lines: Line[]; typed: string; working: boolean; promptRef: React.Ref<HTMLDivElement> }) {
  const { name, prompt, bullet } = LOOKS[look];
  return (
    <div className={`${s.window} ${s.term}`} data-agent={look}>
      <div className={s.termBar} aria-hidden="true"><span>{name}</span></div>
      <div className={s.termBody}>
        <p className={s.hello}><b>{name}</b> <span>{folder}</span></p>
        <div className={s.lines} aria-live="polite">
          {lines.map((line, i) => (
            <p key={i} className={s[line.who]}>
              <i aria-hidden="true">{line.who === "you" ? prompt : bullet}</i>
              <span>{line.text}</span>
            </p>
          ))}
          {working && <p className={s.working}><i aria-hidden="true">{bullet}</i><span>Working</span></p>}
        </div>
        <div ref={promptRef} className={s.prompt} data-agent-prompt="" data-filled={typed ? "" : undefined}>
          <i aria-hidden="true">{prompt}</i>
          <span key={typed ? "typed" : "empty"} className={typed ? s.typed : s.caret}>{typed}</span>
        </div>
      </div>
    </div>
  );
}
