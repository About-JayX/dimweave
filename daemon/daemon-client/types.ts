import type { BridgeMessage } from "../types";
import type { DaemonStatus } from "../control-protocol";

export interface DaemonClientEvents {
  routedMessage: [BridgeMessage];
  disconnect: [];
  status: [DaemonStatus];
}
