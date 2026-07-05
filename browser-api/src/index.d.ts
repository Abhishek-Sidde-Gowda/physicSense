// navigator.physicSense — TypeScript definitions

export type PhysicSenseModality = "wifi" | "acoustic" | "neuromotor";

export type TremorClass =
  | "none"
  | "physiological"
  | "essential"
  | "parkinsonian"
  | "indeterminate";

export interface PhysicSenseSessionInit {
  modalities: PhysicSenseModality[];
  sampleRate?: number;
  wifiCpiSize?: number;
}

export interface PhysicSenseNeuromotor {
  readonly tremorDominantHz: number;
  readonly tremorClass: TremorClass;
  readonly gaitCadenceSpm: number;
  readonly gaitAsymmetryIndex: number;
  readonly updrsProxyScore: number;
  readonly flagForReview: boolean;
}

export interface PhysicSenseFrame {
  readonly timestamp: DOMHighResTimeStamp;
  readonly rangeDopplerMap: Float32Array | null;
  readonly velocityMs: number | null;
  readonly rangeM: number | null;
  readonly position2d: Float32Array | null;
  readonly neuromotor: PhysicSenseNeuromotor | null;
}

export interface PhysicSenseFrameEvent extends Event {
  readonly frame: PhysicSenseFrame;
}

export interface PhysicSenseSession extends EventTarget {
  onframe: ((event: PhysicSenseFrameEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  readonly active: boolean;
  start(): void;
  stop(): void;
}

export interface PhysicSensePermissionDescriptor {
  name:
    | "physicSense.wifi"
    | "physicSense.acoustic"
    | "physicSense.neuromotor";
}

export interface PhysicSense extends EventTarget {
  requestSession(init: PhysicSenseSessionInit): Promise<PhysicSenseSession>;
  queryPermission(
    descriptor: PhysicSensePermissionDescriptor
  ): Promise<PermissionState>;
}

declare global {
  interface Navigator {
    readonly physicSense: PhysicSense;
  }
}
