// Northwind: the made-up invoicing app the demos point at (the same page as
// desktop/lab/northwind.html). Its ids and class names are plain on purpose: they are what
// Clipframes reads and copies, the way it would in a real project.

import "./northwind.css";

const INVOICES = [
  ["INV-1042", "Maersk Line", "overdue", "Sep 12", "$3,120.00"],
  ["INV-1041", "Cellmark AB", "due", "Oct 02", "$8,400.00"],
  ["INV-1040", "Movia Care", "paid", "Sep 28", "$1,950.00"],
  ["INV-1039", "Nyberg Bil", "paid", "Sep 26", "$6,300.00"],
  ["INV-1038", "Ekvia Group", "due", "Oct 05", "$4,540.00"],
  ["INV-1037", "Vanta Studio", "paid", "Sep 21", "$2,780.00"],
  ["INV-1036", "Plex Tech", "paid", "Sep 18", "$12,200.00"],
] as const;

const STATUS = { overdue: "Overdue", due: "Due", paid: "Paid" } as const;

/// Nothing in here takes the keyboard: the page is something to point at. `menu` is the Export
/// menu, which opens in the wrong place: the fault the screen clip demo records.
/// `simple` is the page with less on it, for the first screen: no sidebar and four invoices.
export default function Northwind({ menu, onExport, simple = false }: { menu: boolean; onExport: () => void; simple?: boolean }) {
  return (
    <div className={simple ? "app simple" : "app"} id="app">
      <aside>
        <div className="logo"><i />Northwind</div>
        <nav>
          <a>Overview</a>
          <a className="on">Invoices</a>
          <a>Customers</a>
          <a>Reports</a>
          <a>Settings</a>
        </nav>
      </aside>
      <main>
        <div className="top">
          <div><h1>Invoices</h1><div className="sub">September 2026 · 128 invoices</div></div>
          <div className="actions">
            <input type="search" id="invoice-search" placeholder="Search invoices" aria-label="Search invoices" tabIndex={-1} readOnly />
            <button id="export" className="btn" tabIndex={-1} onClick={onExport}>Export</button>
            <button id="new-invoice" className="btn btn-primary" tabIndex={-1}>New invoice</button>
          </div>
          {menu && (
            <div className="menu" id="export-menu">
              <a>Download as CSV</a>
              <a>Download as PDF</a>
              <a>Send to accountant</a>
            </div>
          )}
        </div>
        <div className="stats">
          <div className="stat" id="paid-total"><span>Paid this month</span> <strong>$48,210</strong></div>
          <div className="stat" id="outstanding-total"><span>Outstanding</span> <strong>$12,940</strong></div>
          <div className="stat overdue" id="overdue-total"><span>Overdue</span> <strong>$3,120</strong></div>
        </div>
        <table id="invoice-table">
          <thead><tr><th>Invoice</th><th className="customer">Customer</th><th>Status</th><th className="due">Due</th><th className="amount">Amount</th></tr></thead>
          <tbody>
            {(simple ? INVOICES.slice(0, 4) : INVOICES).map(([id, customer, status, due, amount]) => (
              // The space after each value keeps a row's text readable when Clipframes names it.
              <tr key={id}>
                <td>{id} </td>
                <td className="customer">{customer} </td>
                <td><span className={`badge ${status}`}>{STATUS[status]}</span> </td>
                <td className="due">{due} </td>
                <td className="amount">{amount}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </main>
    </div>
  );
}
