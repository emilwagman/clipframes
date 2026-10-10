// How Clipframes compares with what people use today. It is apart from page.tsx so the
// preview build can draw it too.
//
// Everything said about Agentation here is from its own site, read on 2026-10-10: the install
// command and component from agentation.com/install, the shape of its copied output (the
// "Standard" format) from agentation.com/output, and what that output can hold from its home
// page. The request shown is ours, set in that documented shape; it is not a capture of
// Agentation running. Change this page when that site changes.

import Footer from "../Footer";
import Header from "../Header";
import s from "./compare.module.css";

export function ComparePage({ stars, preview = false }: { stars: number | null; preview?: boolean }) {
  return (
    <>
      <Header stars={stars} refresh={!preview} here="/compare" />
      <main className={s.page}>
        <h1>One request, three ways.</h1>
        <p className={s.ask}>You want the second of four Paid badges grey. This is what your agent is handed.</p>

        <div className={s.ways}>
          <figure>
            <div className={s.card}>
              <div className={s.shot} aria-label="A small picture of a table with four green badges that all say Paid.">
                {["Paid", "Due", "Paid", "Paid", "Paid"].map((word, i) => <i key={i} data-due={word === "Due" ? "" : undefined}>{word}</i>)}
              </div>
              <p className={s.mono}><span>[Image #1]</span> make the second paid badge grey</p>
            </div>
            <figcaption><b>A screenshot.</b> A picture and your words. The agent finds the badge in the code by itself.</figcaption>
          </figure>
          <figure>
            <div className={s.card}>
              <p className={s.mono}>
                <span>## Page Feedback: /invoices</span>{"\n"}
                <span>### 1. span.badge.paid</span>{"\n"}
                <b>**Location:**</b> `#invoice-table &gt; tbody &gt; tr &gt; td &gt; span.badge.paid`{"\n"}
                <b>**Source:**</b> src/components/InvoiceTable.tsx:31:9{"\n"}
                <b>**React:**</b> `&lt;App&gt; &lt;Invoices&gt; &lt;InvoiceTable&gt; &lt;StatusBadge&gt;`{"\n"}
                <b>**Feedback:**</b> make this one grey
              </p>
            </div>
            <figcaption><b>Agentation.</b> A toolbar you add to your React app. It knows the project: the source file and the component.</figcaption>
          </figure>
          <figure>
            <div className={`${s.card} ${s.ours}`}>
              <p className={s.mono}>[Text &quot;Paid&quot; (#invoice-table .badge.paid), <b>2nd of 4 on the page</b>, under heading &quot;Invoices&quot; in Google Chrome &quot;Invoices&quot;: make this one grey]</p>
            </div>
            <figcaption><b>Clipframes.</b> An app on your computer. It names the thing in any site or app, with nothing added to your project.</figcaption>
          </figure>
        </div>

        <div className={s.truths}>
          <p>Agentation tells the agent more about a React project than Clipframes can.</p>
          <p>Clipframes works where there is no project to add a toolbar to: any site, and desktop apps.</p>
          <p>A screenshot works everywhere and names nothing.</p>
        </div>
        <p className={s.note}>
          Agentation&apos;s part is set in the output format its site documents, with our own example in it. Source: <a href="https://www.agentation.com/output">agentation.com/output</a> and <a href="https://www.agentation.com/install">agentation.com/install</a>, read on 10 October 2026.
        </p>
      </main>
      <Footer />
    </>
  );
}
