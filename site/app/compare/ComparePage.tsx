// How Clipframes compares with what people use today. It is apart from page.tsx so the
// preview build can draw it too.
//
// Everything said about Agentation here is from its own site (agentation.com, its Install and
// Features pages), read on 2026-10-10. Change this page when that changes.

import Footer from "../Footer";
import Header from "../Header";
import s from "./compare.module.css";

const yes = <span className={s.yes} aria-label="Yes">●</span>;
const no = <span className={s.no} aria-label="No">–</span>;

export function ComparePage({ stars, preview = false }: { stars: number | null; preview?: boolean }) {
  return (
    <>
      <Header stars={stars} refresh={!preview} here="/compare" />
      <main className={s.page}>
        <h1>Three ways to tell an agent which button.</h1>

        <div className={s.ways}>
          <figure>
            <div className={s.card}>
              <div className={s.shot} aria-hidden="true"><i /><i /><i /><i /></div>
              <p className={s.mono}>[Image #1] make the paid one grey</p>
            </div>
            <figcaption><b>A screenshot.</b> The agent gets a picture and works out the rest.</figcaption>
          </figure>
          <figure>
            <div className={s.card}>
              <p className={s.mono}>npm install agentation -D</p>
              <p className={s.mono}>{"<Agentation />"}</p>
            </div>
            <figcaption><b>Agentation.</b> A toolbar you add to your React app. It works inside that app.</figcaption>
          </figure>
          <figure>
            <div className={`${s.card} ${s.ours}`}>
              <p className={s.mono}>[Text &quot;Paid&quot; (#invoice-table .badge.paid), 2nd of 4 on the page: make this one grey]</p>
            </div>
            <figcaption><b>Clipframes.</b> An app on your computer. It works in every app.</figcaption>
          </figure>
        </div>

        <table className={s.table}>
          <thead>
            <tr><th scope="col"><span className={s.hide}>Where it works</span></th><th scope="col">A screenshot</th><th scope="col">Agentation</th><th scope="col">Clipframes</th></tr>
          </thead>
          <tbody>
            <tr><th scope="row">The React web app you are building</th><td>{yes}</td><td>{yes}</td><td>{yes}</td></tr>
            <tr><th scope="row">Any other site, with nothing added to it</th><td>{yes}</td><td>{no}</td><td>{yes}</td></tr>
            <tr><th scope="row">Native desktop apps</th><td>{yes}</td><td>{no}</td><td>{yes}</td></tr>
            <tr><th scope="row">Names the element in words the agent can search for</th><td>{no}</td><td>{yes}</td><td>{yes}</td></tr>
            <tr><th scope="row">Source file paths and the React component tree</th><td>{no}</td><td>{yes}</td><td>{no}</td></tr>
          </tbody>
        </table>
        <p className={s.note}>
          Agentation knows more about a React project than Clipframes does. Clipframes works where there is no project to
          add a toolbar to. What is said about Agentation is from <a href="https://www.agentation.com">agentation.com</a>,
          read on 10 October 2026.
        </p>
      </main>
      <Footer />
    </>
  );
}
