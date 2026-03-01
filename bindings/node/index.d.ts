/* Auto-maintained type declarations for the winpane Node.js addon.
 * This file is committed so TypeScript examples resolve without running
 * `npm run build` first.  Keep in sync with bindings/node/src/lib.rs.
 */

export interface HudOptions {
  width: number;
  height: number;
  x?: number;
  y?: number;
  monitor?: number;
  anchor?: 'top_left' | 'top_right' | 'bottom_left' | 'bottom_right';
  margin?: number;
}

export interface PanelOptions {
  width: number;
  height: number;
  x?: number;
  y?: number;
  monitor?: number;
  anchor?: 'top_left' | 'top_right' | 'bottom_left' | 'bottom_right';
  margin?: number;
  draggable?: boolean;
  dragHeight?: number;
}

export interface PipOptions {
  sourceHwnd: number;
  width: number;
  height: number;
  x?: number;
  y?: number;
  monitor?: number;
  anchor?: 'top_left' | 'top_right' | 'bottom_left' | 'bottom_right';
  margin?: number;
}

export interface TrayOptions {
  iconPath?: string;
  tooltip?: string;
}

export interface TextOptions {
  text: string;
  x: number;
  y: number;
  fontSize: number;
  color?: string;
  fontFamily?: string;
  bold?: boolean;
  italic?: boolean;
  interactive?: boolean;
}

export interface RectOptions {
  x: number;
  y: number;
  width: number;
  height: number;
  fill?: string;
  cornerRadius?: number;
  borderColor?: string;
  borderWidth?: number;
  interactive?: boolean;
}

export interface ImageOptions {
  path: string;
  x: number;
  y: number;
  width: number;
  height: number;
  interactive?: boolean;
}

export interface MenuItemOptions {
  id: number;
  label: string;
  enabled?: boolean;
}

export interface SourceRegionOptions {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WinPaneEvent {
  eventType: string;
  surfaceId?: number;
  key?: string;
  button?: string;
  itemId?: number;
}

export class WinPane {
  constructor();

  // Surface creation
  createHud(options: HudOptions): number;
  createPanel(options: PanelOptions): number;
  createPip(options: PipOptions): number;
  createTray(options: TrayOptions): number;

  // Elements
  setText(surfaceId: number, key: string, options: TextOptions): void;
  setRect(surfaceId: number, key: string, options: RectOptions): void;
  setImage(surfaceId: number, key: string, options: ImageOptions): void;
  removeElement(surfaceId: number, key: string): void;

  // Visibility
  show(surfaceId: number): void;
  hide(surfaceId: number): void;

  // Geometry & appearance
  setPosition(surfaceId: number, x: number, y: number): void;
  setSize(surfaceId: number, width: number, height: number): void;
  setOpacity(surfaceId: number, opacity: number): void;
  fadeIn(surfaceId: number, durationMs: number): void;
  fadeOut(surfaceId: number, durationMs: number): void;
  setCaptureExcluded(surfaceId: number, excluded: boolean): void;
  setBackdrop(surfaceId: number, backdrop: string): void;
  backdropSupported(): boolean;

  // Anchoring
  anchorTo(surfaceId: number, targetHwnd: number, anchor: string, offsetX: number, offsetY: number): void;
  unanchor(surfaceId: number): void;

  // PiP
  setSourceRegion(surfaceId: number, options: SourceRegionOptions): void;
  clearSourceRegion(surfaceId: number): void;

  // Tray
  setTooltip(trayId: number, tooltip: string): void;
  setTrayIcon(trayId: number, iconPath: string): void;
  setPopup(trayId: number, panelSurfaceId: number): void;
  setMenu(trayId: number, items: MenuItemOptions[]): void;

  // Events
  pollEvent(): WinPaneEvent | null;

  // Lifecycle
  destroy(id: number): void;
  close(): void;
}
