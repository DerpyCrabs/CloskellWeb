import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";

const zones = [
  { id: 1, name: "Zone 1", min: 90, max: 110, color: "#2a9d8f" },
  { id: 2, name: "Zone 2", min: 111, max: 130, color: "#70b62c" },
  { id: 3, name: "Zone 3", min: 131, max: 150, color: "#f0b429" },
  { id: 4, name: "Zone 4", min: 151, max: 170, color: "#f77f00" },
  { id: 5, name: "Zone 5", min: 171, max: 190, color: "#d9184b" }
];

const entries = [
  {
    id: "intervals",
    startedAt: 1704499200000,
    stoppedAt: 1704500160000,
    durationMs: 960000,
    readings: [
      { bpm: 112, time: 0 },
      { bpm: 151, time: 120000 },
      { bpm: 170, time: 420000 },
      { bpm: 128, time: 900000 }
    ],
    targetZoneId: 4,
    zones,
    exerciseType: "HIIT",
    hiddenAt: null
  },
  {
    id: "steady",
    startedAt: 1704412800000,
    stoppedAt: 1704416400000,
    durationMs: 3600000,
    readings: [
      { bpm: 118, time: 0 },
      { bpm: 124, time: 900000 },
      { bpm: 127, time: 1800000 },
      { bpm: 122, time: 3300000 }
    ],
    targetZoneId: 2,
    zones,
    exerciseType: "LISS",
    hiddenAt: null
  }
];

test.beforeEach(async ({ page }) => {
  await page.addInitScript(({ zonesPayload, entriesPayload }) => {
    localStorage.setItem(
      "heartRateExercise.zones.v1",
      JSON.stringify({ zones: zonesPayload, targetZoneId: 3 })
    );
    localStorage.setItem(
      "heartRateExercise.log.v1",
      JSON.stringify({ version: 2, entries: entriesPayload })
    );
  }, { zonesPayload: zones, entriesPayload: entries });
});

async function installFakeBluetooth(page: Page) {
  await page.addInitScript(() => {
    let readingListener: ((event: { target: { value: DataView } }) => void) | null = null;
    let disconnectListener: (() => void) | null = null;
    const state = {
      requested: false,
      stoppedNotifications: false,
      disconnected: false
    };

    const characteristic = {
      async startNotifications() {
        return characteristic;
      },
      async stopNotifications() {
        state.stoppedNotifications = true;
        return characteristic;
      },
      addEventListener(type: string, listener: (event: { target: { value: DataView } }) => void) {
        if (type === "characteristicvaluechanged") readingListener = listener;
      },
      removeEventListener(type: string) {
        if (type === "characteristicvaluechanged") readingListener = null;
      }
    };

    const device = {
      name: "Polar Test Strap",
      gatt: {
        connected: false,
        async connect() {
          device.gatt.connected = true;
          return {
            async getPrimaryService(service: string) {
              if (service !== "heart_rate") throw new Error(`unexpected service ${service}`);
              return {
                async getCharacteristic(characteristicName: string) {
                  if (characteristicName !== "heart_rate_measurement") {
                    throw new Error(`unexpected characteristic ${characteristicName}`);
                  }
                  return characteristic;
                }
              };
            }
          };
        },
        disconnect() {
          state.disconnected = true;
          device.gatt.connected = false;
          disconnectListener?.();
        }
      },
      addEventListener(type: string, listener: () => void) {
        if (type === "gattserverdisconnected") disconnectListener = listener;
      },
      removeEventListener(type: string) {
        if (type === "gattserverdisconnected") disconnectListener = null;
      }
    };

    Object.defineProperty(navigator, "bluetooth", {
      configurable: true,
      value: {
        async requestDevice(options: { filters?: Array<{ services?: string[] }> }) {
          if (options.filters?.[0]?.services?.[0] !== "heart_rate") {
            throw new Error("heart_rate filter was not requested");
          }
          state.requested = true;
          return device;
        }
      }
    });

    window.__fakeHeartRate = {
      state,
      emit(bpm: number) {
        const bytes = new Uint8Array([0, bpm]);
        readingListener?.({ target: { value: new DataView(bytes.buffer) } });
      }
    };
  });
}

async function installFailingHostAPIs(page: Page) {
  await page.addInitScript(() => {
    const host = window as typeof window & {
      __hrwebStorageFailures?: Set<string>;
      __hrwebDownloadFails?: boolean;
    };
    host.__hrwebStorageFailures = new Set();
    host.__hrwebDownloadFails = false;

    const originalSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function(key: string, value: string) {
      if (host.__hrwebStorageFailures?.has(key)) {
        throw new Error(`Storage denied for ${key}`);
      }
      return originalSetItem.call(this, key, value);
    };

    const originalCreateObjectURL = URL.createObjectURL.bind(URL);
    URL.createObjectURL = function(value: Blob | MediaSource) {
      if (host.__hrwebDownloadFails) {
        throw new Error("Download blocked");
      }
      return originalCreateObjectURL(value);
    };
  });
}

async function canvasDataUrl(page: Page, testId: string) {
  return page.evaluate((id) => {
    const canvas = document.querySelector(`[data-testid="${id}"]`) as HTMLCanvasElement | null;
    return canvas?.toDataURL() || "";
  }, testId);
}

async function expectCanvasInk(page: Page, testId: string) {
  await expect(page.getByTestId(testId)).toBeVisible();
  await page.waitForFunction((id) => {
    const canvas = document.querySelector(`[data-testid="${id}"]`) as HTMLCanvasElement | null;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx || canvas.width <= 0 || canvas.height <= 0) return false;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    for (let index = 0; index < data.length; index += 64) {
      const red = data[index] ?? 255;
      const green = data[index + 1] ?? 255;
      const blue = data[index + 2] ?? 255;
      const alpha = data[index + 3] ?? 0;
      if (alpha > 0 && !(red > 248 && green > 248 && blue > 245)) return true;
    }
    return false;
  }, testId);
}

async function expectCanvasBacksRenderedSize(page: Page, testId: string) {
  await expect.poll(async () =>
    page.evaluate((id) => {
      const canvas = document.querySelector(`[data-testid="${id}"]`) as HTMLCanvasElement | null;
      if (!canvas) return false;

      const rect = canvas.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      const expectedWidth = Math.round(rect.width * ratio);
      const expectedHeight = Math.round(rect.height * ratio);
      return (
        Math.abs(canvas.width - expectedWidth) <= 2 &&
        Math.abs(canvas.height - expectedHeight) <= 2
      );
    }, testId)
  ).toBe(true);
}

async function dispatchPointer(page: Page, testId: string, type: string) {
  await page.getByTestId(testId).dispatchEvent(type, {
    bubbles: true,
    cancelable: true,
    pointerId: 1,
    pointerType: "mouse"
  });
}

async function connectSimulatedMonitor(page: Page) {
  await page.keyboard.press("Control+Shift+H");
  await expect(page.getByTestId("connection-status")).toContainText("Simulated");
}

async function holdStopWorkout(page: Page) {
  const stopButton = page.getByTestId("stop-workout");
  await dispatchPointer(page, "stop-workout", "pointerdown");
  await expect(stopButton).toHaveAttribute("data-holding", "");
  await expect(stopButton).toContainText("Hold...");
  await expect.poll(async () => page.getByTestId("hrweb-app").getAttribute("data-state"), {
    timeout: 2500
  }).toBe("idle");
}

test("boots stored HRWeb state and renders log metrics", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "idle");
  await expect(page.getByTestId("status-pill")).toContainText("Idle");

  await page.getByRole("button", { name: "Log" }).click();
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "2");
  await expect(page.getByTestId("log-type-filter")).toHaveValue("");
  await expect(page.getByTestId("log-row-intervals")).toBeVisible();
  await expectCanvasInk(page, "log-heart-chart");
  const hiitLogChart = await canvasDataUrl(page, "log-heart-chart");
  await expect(page.getByTestId("log-hrr")).toHaveCount(0);
  await expect(page.getByTestId("import-status")).toBeHidden();
  await expect(page.getByTestId("zone-time-2")).toContainText("3:00");
  await expect(page.getByTestId("zone-time-2")).toHaveAttribute("data-percent", "19");
  await expect(page.getByTestId("zone-time-4")).toContainText("13:00");
  await expect(page.getByTestId("zone-time-4")).toHaveAttribute("data-percent", "81");

  await page.getByTestId("log-type-filter").selectOption("LISS");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-filter", "LISS");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "1");
  await expect(page.getByTestId("log-row-steady")).toBeVisible();
  await expect(page.getByTestId("log-row-steady")).toHaveAttribute("data-selected", "");
  await expect(page.getByTestId("type-draft")).toHaveValue("LISS");
  await expect(page.getByTestId("log-row-intervals")).toHaveCount(0);
  await expectCanvasInk(page, "log-heart-chart");
  await expect.poll(() => canvasDataUrl(page, "log-heart-chart")).not.toBe(hiitLogChart);
  const lissLogChart = await canvasDataUrl(page, "log-heart-chart");
  await expect(page.getByTestId("log-hrr")).toHaveCount(0);
  await expect(page.getByTestId("zone-time-2")).toContainText("1:00:00");
  await expect(page.getByTestId("zone-time-2")).toHaveAttribute("data-percent", "100");

  await page.getByTestId("log-type-filter").selectOption("HIIT");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-filter", "HIIT");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "1");
  await expect(page.getByTestId("log-row-intervals")).toHaveAttribute("data-selected", "");
  await expect(page.getByTestId("type-draft")).toHaveValue("HIIT");
  await expectCanvasInk(page, "log-heart-chart");
  await expect.poll(() => canvasDataUrl(page, "log-heart-chart")).not.toBe(lissLogChart);

  await page.getByTestId("log-type-filter").selectOption("__untyped__");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-filter", "__untyped__");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "0");
  await expect(page.getByTestId("empty-filtered-log")).toContainText("No exercises match");

  await page.getByTestId("log-type-filter").selectOption("");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-filter", "");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "2");

  await page.getByRole("button", { name: "Metrics" }).click();
  await expect(page.getByTestId("metrics-content")).toHaveClass(/max-h-\[617px\]/);
  await expect(page.getByTestId("metrics-content")).toHaveClass(/overflow-y-auto/);
  await expect(page.getByTestId("metrics-pane")).toHaveAttribute("data-count", "2");
  await expect(page.getByTestId("metrics-pane")).toHaveAttribute("data-grouping", "week");

  const hiitGroup = page.getByTestId("metric-group-HIIT");
  const lissGroup = page.getByTestId("metric-group-LISS");
  await expect(hiitGroup).toContainText("HRR 1 min");
  await expect(hiitGroup).toContainText("TRIMP");
  await expect(hiitGroup).not.toContainText("Zone 2 adherence");
  await expect(lissGroup).toContainText("Zone 2 adherence");
  await expect(lissGroup).toContainText("TRIMP");
  await expect(lissGroup).not.toContainText("HRR 1 min");

  await expectCanvasInk(page, "metric-chart-hrr-HIIT");
  await expectCanvasInk(page, "metric-chart-trimp-HIIT");
  await expectCanvasInk(page, "metric-chart-zone2-LISS");
  await expectCanvasInk(page, "metric-chart-trimp-LISS");
  await expectCanvasBacksRenderedSize(page, "metric-chart-zone2-LISS");
  await expectCanvasBacksRenderedSize(page, "metric-chart-trimp-LISS");
  await expect(hiitGroup.getByTestId("metric-chart-zone2-HIIT")).toHaveCount(0);
  await expect(lissGroup.getByTestId("metric-chart-hrr-LISS")).toHaveCount(0);

  const weeklyZone2Chart = await canvasDataUrl(page, "metric-chart-zone2-LISS");
  await page.getByTestId("metrics-grouping").selectOption("month");
  await expect(page.getByTestId("metrics-pane")).toHaveAttribute("data-grouping", "month");
  await expect(page.getByTestId("metrics-grouping")).toHaveValue("month");
  await expect.poll(() => canvasDataUrl(page, "metric-chart-zone2-LISS")).not.toBe(weeklyZone2Chart);
});

test("draws the log chart with the Solid scale and canvas styling", async ({ page }) => {
  await page.addInitScript(() => {
    const host = window as typeof window & {
      __hrwebCanvasCalls?: Array<Record<string, unknown>>;
    };
    host.__hrwebCanvasCalls = [];

    const proto = CanvasRenderingContext2D.prototype as CanvasRenderingContext2D & {
      __hrwebRecorderInstalled?: boolean;
      [key: string]: unknown;
    };
    if (proto.__hrwebRecorderInstalled) return;
    proto.__hrwebRecorderInstalled = true;

    for (const method of ["strokeRect", "fillText", "stroke", "arc"]) {
      const original = proto[method] as (...args: unknown[]) => unknown;
      proto[method] = function(this: CanvasRenderingContext2D, ...args: unknown[]) {
        const testId = this.canvas?.dataset?.testid;
        if (testId === "log-heart-chart") {
          host.__hrwebCanvasCalls?.push({
            method,
            args,
            testId,
            font: this.font,
            lineWidth: this.lineWidth,
            lineCap: this.lineCap,
            lineJoin: this.lineJoin,
            strokeStyle: this.strokeStyle,
            fillStyle: this.fillStyle,
            globalAlpha: this.globalAlpha
          });
        }
        return original.apply(this, args);
      };
    }
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Log" }).click();
  await expectCanvasInk(page, "log-heart-chart");

  const report = await page.evaluate((testZones) => {
    const calls = ((window as typeof window & {
      __hrwebCanvasCalls?: Array<{
        method: string;
        args: unknown[];
        font: string;
        lineWidth: number;
        lineCap: string;
      }>;
    }).__hrwebCanvasCalls || []);
    const canvas = document.querySelector("[data-testid='log-heart-chart']") as HTMLCanvasElement | null;
    if (!canvas) throw new Error("log chart canvas was not found");

    const labels = calls.filter((call) => call.method === "fillText");
    const boundaryTexts = new Set<string>();
    testZones.forEach((zone, index) => {
      if (index === 0) boundaryTexts.add(String(zone.min));
      boundaryTexts.add(String(zone.max));
    });

    const targetRects = calls.filter((call) => call.method === "strokeRect");
    const targetRect = targetRects[targetRects.length - 1];
    if (!targetRect) throw new Error("target zone stroke was not drawn");

    const rect = canvas.getBoundingClientRect();
    const cssWidth = rect.width;
    const cssHeight = rect.height;
    const basePadding = Math.max(34, Math.round(cssWidth * 0.04));
    const xLabelPx = Math.max(12, Math.round(cssWidth / 52));
    const top = basePadding * 0.55;
    const bottom = cssHeight - (basePadding + xLabelPx * 1.55);
    const plotHeight = bottom - top;
    const yMin = 35;
    const yMax = 205;
    const yForBpm = (bpm: number) => bottom - ((bpm - yMin) / (yMax - yMin)) * plotHeight;
    const targetZoneIndex = testZones.findIndex((zone) => zone.id === 4);
    const targetZone = testZones[targetZoneIndex];
    const previousZone = testZones[targetZoneIndex - 1];
    const nextZone = testZones[targetZoneIndex + 1];
    const lowerBoundary = previousZone ? (previousZone.max + targetZone.min) / 2 : targetZone.min;
    const upperBoundary = nextZone ? (targetZone.max + nextZone.min) / 2 : targetZone.max;
    const bandTop = Math.max(top, Math.floor(yForBpm(upperBoundary)));
    const bandBottom = Math.min(bottom, Math.ceil(yForBpm(lowerBoundary)));

    return {
      boundaryFonts: labels
        .filter((call) => boundaryTexts.has(String(call.args[0])))
        .map((call) => call.font),
      axisFonts: labels
        .filter((call) => String(call.args[0]).endsWith("m"))
        .map((call) => call.font),
      readingLineWidths: calls
        .filter((call) => call.method === "stroke" && call.lineCap === "round")
        .map((call) => call.lineWidth),
      targetY: Number(targetRect.args[1]),
      targetHeight: Number(targetRect.args[3]),
      expectedY: bandTop,
      expectedHeight: Math.max(1, bandBottom - bandTop)
    };
  }, zones);

  expect(report.boundaryFonts.length).toBeGreaterThan(0);
  expect(report.boundaryFonts.every((font) => !/^([5-9]\d\d|bold)\b/.test(font))).toBe(true);
  expect(report.axisFonts.length).toBeGreaterThan(0);
  expect(report.axisFonts.every((font) => /^600\b/.test(font))).toBe(true);
  expect(report.readingLineWidths.length).toBeGreaterThanOrEqual(3);
  expect(new Set(report.readingLineWidths)).toEqual(new Set([3]));
  expect(Math.abs(report.targetY - report.expectedY)).toBeLessThanOrEqual(1);
  expect(Math.abs(report.targetHeight - report.expectedHeight)).toBeLessThanOrEqual(1);
});

test("runs the dev simulator and saves a workout", async ({ page }) => {
  await page.goto("/");

  await connectSimulatedMonitor(page);
  await expect(page.getByTestId("latest-bpm")).not.toContainText("--");

  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
  await page.waitForTimeout(1200);

  const stopButton = page.getByTestId("stop-workout");
  await dispatchPointer(page, "stop-workout", "pointerdown");
  await expect(stopButton).toHaveAttribute("data-holding", "");
  await page.waitForTimeout(150);
  await dispatchPointer(page, "stop-workout", "pointerup");
  await expect(stopButton).not.toHaveAttribute("data-holding", "");
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");

  await holdStopWorkout(page);

  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "idle");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "3");
  await expect(page.getByTestId("type-picker-input")).toBeFocused();
  await expect(page.getByTestId("type-picker-input")).toHaveValue("LISS");
  await page.getByTestId("type-picker-input").fill("Tempo");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("type-picker")).toHaveCount(0);
  await expect(page.getByTestId("type-draft")).toHaveValue("Tempo");

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  expect(saved.entries).toHaveLength(3);
  expect(saved.entries[0].readings.length).toBeGreaterThan(0);
  expect(saved.entries[0].exerciseType).toBe("Tempo");
});

test("keyboard hold stop cancels and completes a workout", async ({ page }) => {
  await page.goto("/");

  await connectSimulatedMonitor(page);
  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
  await page.waitForTimeout(1200);

  const stopButton = page.getByTestId("stop-workout");
  await stopButton.focus();
  await expect(stopButton).toBeFocused();

  await page.keyboard.down("Enter");
  await expect(stopButton).toHaveAttribute("data-holding", "");
  await page.waitForTimeout(150);
  await page.keyboard.up("Enter");
  await expect(stopButton).not.toHaveAttribute("data-holding", "");
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");

  await page.keyboard.down("Space");
  await expect(stopButton).toHaveAttribute("data-holding", "");
  await expect.poll(async () => page.getByTestId("hrweb-app").getAttribute("data-state"), {
    timeout: 2500
  }).toBe("idle");
  await page.keyboard.up("Space");
  await expect(page.getByTestId("type-picker-input")).toBeFocused();
});

test("paused workout offers Resume instead of restarting", async ({ page }) => {
  await page.goto("/");

  await connectSimulatedMonitor(page);
  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
  await page.waitForTimeout(500);

  await page.getByRole("button", { name: "Pause" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "paused");
  await expect(page.getByRole("button", { name: "Start" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Resume" })).toBeEnabled();

  await page.getByRole("button", { name: "Resume" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
});

test("blocks workout controls without a connection and toggles the dev simulator hotkey", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Start" })).toBeDisabled();

  await page.keyboard.press("Control+Shift+H");
  await expect(page.getByTestId("connection-status")).toContainText("Simulated");
  await expect(page.getByRole("button", { name: "Start" })).toBeEnabled();

  await page.keyboard.press("Control+Shift+H");
  await expect(page.getByTestId("connection-status")).toContainText("Disconnected");
  await expect(page.getByRole("button", { name: "Start" })).toBeDisabled();
});

test("updates live data without replacing stable DOM nodes", async ({ page }) => {
  await page.goto("/");

  await page.evaluate(() => {
    const root = document.querySelector("[data-testid='hrweb-app']");
    window.__hrwebRefs = {
      root,
      topbar: document.querySelector(".topbar"),
      bpmText: document.querySelector("[data-testid='latest-bpm'] strong"),
      connectionText: document.querySelector("[data-testid='connection-status']")
    };
  });

  await connectSimulatedMonitor(page);
  await expect(page.getByTestId("latest-bpm")).not.toContainText("--");

  const preserved = await page.evaluate(() => {
    const refs = window.__hrwebRefs;
    return {
      root: refs.root === document.querySelector("[data-testid='hrweb-app']"),
      topbar: refs.topbar === document.querySelector(".topbar"),
      bpmText: refs.bpmText === document.querySelector("[data-testid='latest-bpm'] strong"),
      connectionText:
        refs.connectionText === document.querySelector("[data-testid='connection-status']")
    };
  });

  expect(preserved).toEqual({
    root: true,
    topbar: true,
    bpmText: true,
    connectionText: true
  });
});

test("draws and updates the live heart chart canvas", async ({ page }) => {
  await installFakeBluetooth(page);
  await page.goto("/");

  const canvas = page.getByTestId("live-heart-chart");
  await expect(canvas).toBeVisible();
  await expect.poll(async () =>
    page.evaluate(() => {
      const canvas = document.querySelector("[data-testid='live-heart-chart']") as HTMLCanvasElement | null;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx || canvas.width <= 0 || canvas.height <= 0) return false;
      const pixel = ctx.getImageData(Math.floor(canvas.width / 2), Math.floor(canvas.height / 2), 1, 1).data;
      return pixel[3] > 0;
    })
  ).toBe(true);

  const before = await page.evaluate(() =>
    (document.querySelector("[data-testid='live-heart-chart']") as HTMLCanvasElement).toDataURL()
  );

  await page.getByRole("button", { name: "Connect monitor" }).click();
  await page.getByRole("button", { name: "Start" }).click();
  await page.evaluate(() => {
    window.__fakeHeartRate.emit(142);
    window.__fakeHeartRate.emit(148);
  });
  await expect(page.getByTestId("latest-bpm")).toContainText("148");

  await expect.poll(async () =>
    page.evaluate((previous) =>
      (document.querySelector("[data-testid='live-heart-chart']") as HTMLCanvasElement).toDataURL() !== previous,
      before
    )
  ).toBe(true);
});

test("connects to a Bluetooth heart-rate monitor and records readings", async ({ page }) => {
  await installFakeBluetooth(page);
  await page.goto("/");

  await page.getByRole("button", { name: "Connect monitor" }).click();
  await expect(page.getByTestId("connection-status")).toContainText("Bluetooth");
  await expect(page.getByTestId("status-pill")).toContainText("Bluetooth");
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute(
    "data-message",
    "Heart-rate monitor connected"
  );
  await page.evaluate(() => {
    window.__fakeHeartRate.emit(101);
    window.__fakeHeartRate.emit(104);
  });
  await expect(page.getByTestId("latest-bpm")).toContainText("104");

  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
  await page.evaluate(() => window.__fakeHeartRate.emit(142));
  await expect(page.getByTestId("latest-bpm")).toContainText("142");

  await holdStopWorkout(page);
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "idle");

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  expect(saved.entries[0].readings.at(-1).bpm).toBe(142);
  expect(saved.entries[0].readings.map((reading: { bpm: number }) => reading.bpm)).toEqual([142]);

  await page.getByRole("button", { name: "Disconnect" }).click();
  await expect(page.getByTestId("connection-status")).toContainText("Disconnected");
  const bluetoothState = await page.evaluate(() => window.__fakeHeartRate.state);
  expect(bluetoothState.requested).toBe(true);
  expect(bluetoothState.disconnected).toBe(true);
});

test("disconnect pauses an active Bluetooth workout", async ({ page }) => {
  await installFakeBluetooth(page);
  await page.goto("/");

  await page.getByRole("button", { name: "Connect monitor" }).click();
  await expect(page.getByTestId("connection-status")).toContainText("Bluetooth");
  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "running");
  await page.evaluate(() => window.__fakeHeartRate.emit(145));
  await expect(page.getByTestId("latest-bpm")).toContainText("145");

  await page.getByRole("button", { name: "Disconnect" }).click();
  await expect(page.getByTestId("connection-status")).toContainText("Disconnected");
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-state", "paused");
  await expect(page.getByRole("button", { name: "Resume" })).toBeDisabled();

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  expect(saved.entries).toHaveLength(2);
});

test("persists zone edits and selected target", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Zones" })).toHaveCount(0);
  const zone2Handle = page.getByTestId("boundary-2");
  for (let step = 0; step < 3; step += 1) {
    await zone2Handle.focus();
    await page.keyboard.press("ArrowRight");
  }
  await expect(page.getByTestId("zone-2-max")).toHaveText("133");
  await page.getByTestId("target-zone-4").click();
  const track = page.getByTestId("zone-boundary-track");
  const handle = page.getByTestId("boundary-3");
  const trackBox = await track.boundingBox();
  const handleBox = await handle.boundingBox();
  if (!trackBox || !handleBox) throw new Error("zone boundary geometry was not available");

  await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(trackBox.x + trackBox.width * 0.72, trackBox.y + trackBox.height / 2);
  await page.mouse.up();
  await expect(page.getByTestId("zone-3-max")).toHaveText("162");

  await handle.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.getByTestId("zone-3-max")).toHaveText("161");

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.zones.v1") || "null")
  );
  expect(saved.targetZoneId).toBe(4);
  expect(saved.zones[1].max).toBe(133);
  expect(saved.zones[2].max).toBe(161);
  expect(saved.zones[3].min).toBe(162);
});

test("surfaces save and export failures without losing UI state", async ({ page }) => {
  await installFailingHostAPIs(page);
  await page.goto("/");

  await page.evaluate(() =>
    window.__hrwebStorageFailures?.add("heartRateExercise.zones.v1")
  );
  await page.getByTestId("target-zone-4").click();
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-status", "warn");
  await expect(page.getByTestId("status-pill")).toContainText("Save failed");
  await expect(page.getByTestId("zones-pane")).toHaveAttribute(
    "data-status",
    "Storage denied for heartRateExercise.zones.v1"
  );
  await expect(page.getByTestId("target-zone-4")).toHaveAttribute("data-selected", "");

  await page.evaluate(() =>
    window.__hrwebStorageFailures?.add("heartRateExercise.log.v1")
  );
  await page.getByRole("button", { name: "Log" }).click();
  await page.getByTestId("type-draft").fill("Tempo");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByTestId("status-pill")).toContainText("Save failed");
  await expect(page.getByTestId("hrweb-app")).toHaveAttribute(
    "data-message",
    "Storage denied for heartRateExercise.log.v1"
  );
  await expect(page.getByTestId("type-draft")).toHaveValue("Tempo");

  await page.evaluate(() => {
    window.__hrwebDownloadFails = true;
  });
  await page.getByRole("button", { name: "Export" }).click();
  await expect(page.getByTestId("status-pill")).toContainText("Export failed");
  await expect(page.getByTestId("import-status")).toContainText("Download blocked");
});

test("edits workout type and exports the log", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Log" }).click();
  await page.getByTestId("log-row-intervals").click();
  await page.getByTestId("type-draft").fill("Tempo");
  await page.getByRole("button", { name: "Save" }).click();

  await expect.poll(async () =>
    page.evaluate(() => {
      const saved = JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null");
      return saved.entries.find((entry: { id: string }) => entry.id === "intervals")?.exerciseType;
    })
  ).toBe("Tempo");

  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export" }).click();
  const exported = await download;
  expect(exported.suggestedFilename()).toBe(
    `exercise-log-${new Date().toISOString().slice(0, 10)}.json`
  );
  const exportedPath = await exported.path();
  if (!exportedPath) throw new Error("exported log file was not available");
  const payload = JSON.parse(await readFile(exportedPath, "utf8"));
  expect(payload.version).toBe(2);
  expect(payload.entries.map((entry: { id: string }) => entry.id)).toEqual(["intervals", "steady"]);
  expect(payload.entries[0].exerciseType).toBe("Tempo");
});

test("imports a JSON log through the browser file input", async ({ page }) => {
  await page.goto("/");

  const imported = {
    version: 2,
    entries: [
      {
        id: "imported-tempo",
        startedAt: 1704585600000,
        stoppedAt: 1704587400000,
        durationMs: 1800000,
        readings: [
          { bpm: 119.6, time: 0 },
          { bpm: 136.1, time: 900000 }
        ],
        targetZoneId: 3,
        zones,
        exerciseType: " Tempo ",
        hiddenAt: null
      },
      { id: 42, readings: [] }
    ]
  };

  await page.getByRole("button", { name: "Log" }).click();
  const chooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import" }).click();
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: "hrweb-import.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(imported))
  });

  await expect(page.getByTestId("import-status")).toContainText("Replaced log with 1 exercise.");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "1");
  await expect(page.getByTestId("log-row-imported-tempo")).toBeVisible();

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  expect(saved.entries).toHaveLength(1);
  expect(saved.entries[0].exerciseType).toBe("Tempo");
  expect(saved.entries[0].readings.map((reading: { bpm: number }) => reading.bpm)).toEqual([120, 136]);
});

test("rejects invalid JSON log imports without replacing the current log", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Log" }).click();
  const before = await page.evaluate(() => localStorage.getItem("heartRateExercise.log.v1"));

  let chooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import" }).click();
  await (await chooserPromise).setFiles({
    name: "hrweb-import.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ version: 2, workouts: [] }))
  });

  await expect(page.getByTestId("import-status")).toContainText("File does not contain an exercise log.");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "2");
  await expect(page.getByTestId("log-row-intervals")).toBeVisible();
  await expect(page.getByTestId("log-row-steady")).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("heartRateExercise.log.v1"))).toBe(before);

  chooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import" }).click();
  await (await chooserPromise).setFiles({
    name: "hrweb-import.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ version: 2, entries: [{ id: "bad", readings: [] }] }))
  });

  await expect(page.getByTestId("import-status")).toContainText("No valid exercise entries were found.");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "2");
  expect(await page.evaluate(() => localStorage.getItem("heartRateExercise.log.v1"))).toBe(before);
});

test("hold delete hides the selected log and persists it", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Log" }).click();
  const deleteButton = page.getByTestId("delete-log");
  await deleteButton.dispatchEvent("pointerdown", {
    bubbles: true,
    cancelable: true,
    pointerId: 1,
    pointerType: "mouse"
  });
  await expect(deleteButton).toHaveAttribute("data-holding", "");

  await expect.poll(async () => page.getByTestId("log-pane").getAttribute("data-count")).toBe("1");

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  const hidden = saved.entries.find((entry: { id: string }) => entry.id === "intervals");
  const visible = saved.entries.filter((entry: { hiddenAt: number | null }) => entry.hiddenAt == null);
  expect(Number.isFinite(hidden.hiddenAt)).toBe(true);
  expect(visible.map((entry: { id: string }) => entry.id)).toEqual(["steady"]);

  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export" }).click();
  const exported = await download;
  expect(exported.suggestedFilename()).toBe(
    `exercise-log-${new Date().toISOString().slice(0, 10)}.json`
  );
  const exportedPath = await exported.path();
  if (!exportedPath) throw new Error("exported log file was not available");
  const payload = JSON.parse(await readFile(exportedPath, "utf8"));
  expect(payload.entries.map((entry: { id: string }) => entry.id)).toEqual(["steady"]);

  await page.getByTestId("delete-log").dispatchEvent("pointerdown", {
    bubbles: true,
    cancelable: true,
    pointerId: 1,
    pointerType: "mouse"
  });
  await expect.poll(async () => page.getByTestId("log-pane").getAttribute("data-count")).toBe("0");
  await expect(page.getByRole("button", { name: "Export" })).toBeDisabled();
});

test("keyboard hold delete cancels and persists after completion", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: "Log" }).click();
  const deleteButton = page.getByTestId("delete-log");
  await deleteButton.focus();
  await expect(deleteButton).toBeFocused();

  await page.keyboard.down("Enter");
  await expect(deleteButton).toHaveAttribute("data-holding", "");
  await page.waitForTimeout(150);
  await page.keyboard.up("Enter");
  await expect(deleteButton).not.toHaveAttribute("data-holding", "");
  await expect(page.getByTestId("log-pane")).toHaveAttribute("data-count", "2");

  await page.keyboard.down("Space");
  await expect(deleteButton).toHaveAttribute("data-holding", "");
  await expect.poll(async () => page.getByTestId("log-pane").getAttribute("data-count")).toBe("1");
  await page.keyboard.up("Space");

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("heartRateExercise.log.v1") || "null")
  );
  const hidden = saved.entries.find((entry: { id: string }) => entry.id === "intervals");
  const visible = saved.entries.filter((entry: { hiddenAt: number | null }) => entry.hiddenAt == null);
  expect(Number.isFinite(hidden.hiddenAt)).toBe(true);
  expect(visible.map((entry: { id: string }) => entry.id)).toEqual(["steady"]);
});

test("tracks responsive media-query state", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 780 });
  await page.goto("/");

  await expect(page.getByTestId("hrweb-app")).toHaveAttribute("data-mobile", "");
  await expect(page.getByTestId("monitor-heading")).not.toBeVisible();
  await expect(page.getByTestId("latest-bpm")).toHaveClass(/!m-0/);
  await expect(page.getByTestId("exercise-tabs-desktop")).toHaveCount(0);
  await expect(page.getByTestId("exercise-tabs-mobile")).toBeVisible();
  await expect(page.getByTestId("summary-stats")).toHaveClass(/!w-full/);
  expect(await page.evaluate(() => {
    const stats = document.querySelector("[data-testid='summary-stats']");
    const tabs = document.querySelector("[data-testid='exercise-tabs-mobile']");
    return Boolean(stats && tabs && (stats.compareDocumentPosition(tabs) & Node.DOCUMENT_POSITION_FOLLOWING));
  })).toBe(true);
  await page.getByRole("button", { name: "Metrics" }).click();
  await expect(page.getByTestId("metrics-content")).toHaveClass(/overflow-x-hidden/);
  await expect(page.getByTestId("metrics-content")).not.toHaveClass(/max-h-\[617px\]/);

  await page.setViewportSize({ width: 1024, height: 780 });
  await expect(page.getByTestId("hrweb-app")).not.toHaveAttribute("data-mobile", "");
  await expect(page.getByTestId("monitor-heading")).toBeVisible();
  await expect(page.getByTestId("latest-bpm")).not.toHaveClass(/!m-0/);
  await expect(page.getByTestId("exercise-tabs-desktop")).toBeVisible();
  await expect(page.getByTestId("exercise-tabs-mobile")).toHaveCount(0);
  await expect(page.getByTestId("summary-stats")).not.toHaveClass(/!w-full/);
  expect(await page.evaluate(() => {
    const tabs = document.querySelector("[data-testid='exercise-tabs-desktop']");
    const stats = document.querySelector("[data-testid='summary-stats']");
    return Boolean(tabs && stats && (tabs.compareDocumentPosition(stats) & Node.DOCUMENT_POSITION_FOLLOWING));
  })).toBe(true);
  await expect(page.getByTestId("metrics-content")).toHaveClass(/max-h-\[617px\]/);
  await expect(page.getByTestId("metrics-content")).toHaveClass(/overflow-y-auto/);
});

test("keeps desktop component chrome in the narrow stacked layout", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 900 });
  await page.goto("/");

  await expect(page.getByTestId("hrweb-app")).not.toHaveAttribute("data-mobile", "");
  await expect(page.getByTestId("monitor-heading")).toBeVisible();
  await expect(page.getByTestId("exercise-tabs-desktop")).toBeVisible();
  await expect(page.getByTestId("exercise-tabs-mobile")).toHaveCount(0);
  await expect(page.getByTestId("summary-stats")).not.toHaveClass(/!w-full/);
  await expect(page.getByTestId("latest-bpm")).not.toHaveClass(/!m-0/);

  const layout = await page.evaluate(() => {
    const monitor = document.querySelector("[data-testid='connection-pane']")?.getBoundingClientRect();
    const workspace = document.querySelector(".workspace")?.getBoundingClientRect();
    const tabsElement = document.querySelector("[data-testid='exercise-tabs-desktop']");
    const statsElement = document.querySelector("[data-testid='summary-stats']");
    const tabs = tabsElement?.getBoundingClientRect();
    const stats = statsElement?.getBoundingClientRect();
    return {
      stacked: Boolean(monitor && workspace && workspace.top > monitor.bottom),
      tabsBeforeStats: Boolean(tabsElement && statsElement && (tabsElement.compareDocumentPosition(statsElement) & Node.DOCUMENT_POSITION_FOLLOWING)),
      topbarSameRow: Boolean(tabs && stats && Math.abs(tabs.top - stats.top) < 4)
    };
  });

  expect(layout).toEqual({ stacked: true, tabsBeforeStats: true, topbarSameRow: true });
});
