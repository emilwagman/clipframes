import s from "./home.module.css";

/// What a capture actually produced: the line it copied, and the notes.md the agent reads.
/// Both are pasted in unchanged from the capture shown in the recording above.
export default function Output({ reference, notes }: { reference?: string; notes?: string }) {
  return (
    <>
      {reference && <div className={s.ref}><code>{reference}</code></div>}
      {notes && (
        <details className={s.out}>
          <summary>What the agent reads: notes.md from this capture</summary>
          <pre>{notes}</pre>
        </details>
      )}
    </>
  );
}
