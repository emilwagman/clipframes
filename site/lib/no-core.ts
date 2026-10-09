/// Stands in for the desktop core where the app's windows are shown on the site: there is no
/// core to ask, so the components are handed what they show (see app/demo/HistoryDemo.tsx).

export function invoke<T>(command: string, _args?: Record<string, unknown>): Promise<T> {
  void command;
  return Promise.reject(new Error("The website has no desktop core."));
}

export function convertFileSrc(path: string): string {
  return path;
}
