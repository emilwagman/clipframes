// Focus: a made-up timer, drawn as a native desktop window. It is the same app as the real one
// the recording was made in (demo/native/Focus.cs in the app's repository), so what is picked
// here reads as it does there. Its parts have the names the real app gives them and no class
// names: a native window has none to copy.

import "./focus.css";

const SESSIONS = ["09:10  Write the invoice export", "10:05  Review pull requests", "11:30  Fix the search filter"];

/// `changed` is the app after the agent has done what the example asked: the Start button is green.
export default function Focus({ changed = false }: { changed?: boolean }) {
  return (
    <div className="focusapp" id="Focus" data-changed={changed ? "" : undefined}>
      <div id="Layout">
      <div id="Timer">
        <span id="SessionLabel">Deep work</span>
        <span id="TimeLeft">25:00</span>
        <div id="Buttons">
          <button id="StartButton" tabIndex={-1}>Start</button>
          <button id="ResetButton" tabIndex={-1}>Reset</button>
          <button id="SkipButton" tabIndex={-1}>Skip break</button>
        </div>
        <label id="SoundCheck"><i />Play a sound when the time is up</label>
      </div>
      <div id="Today">
        <strong id="TodayHeading">Today</strong>
        <span id="TodayTotal">3 sessions, 1 h 15 min</span>
        <ul id="SessionList">
          {SESSIONS.map((session) => <li key={session}>{session}</li>)}
        </ul>
        <button id="ClearButton" tabIndex={-1}>Clear today</button>
      </div>
      </div>
    </div>
  );
}
