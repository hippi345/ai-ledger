import { exportFileName } from "./csv";

export const pngFileName = (name: string, date: string) =>
  exportFileName(name, date, "png");
