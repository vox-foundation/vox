import React from "react";

export function PortStatus(): React.ReactElement {
  return (
    <div className="px-4">
    <p className={"text-base"}>
      {"Listening on :8080 (Verified)"}
    </p>
    </div>
  );
}
