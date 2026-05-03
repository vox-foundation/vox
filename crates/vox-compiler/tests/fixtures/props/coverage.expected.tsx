import React from "react";

export interface MemberAccessProps {
  items: Array;
}

export function MemberAccess({ items }: MemberAccessProps): React.ReactElement {
  return (
    <span>
      {items.length}
    </span>
  );
}

import React from "react";

export interface NegationProps {
  active: boolean;
}

export function Negation({ active }: NegationProps): React.ReactElement {
  return (
    <span data-disabled={!active}>
      {"label"}
    </span>
  );
}

import React from "react";

export interface ConditionalProps {
  ok: boolean;
  a: string;
  b: string;
}

export function Conditional({ ok, a, b }: ConditionalProps): React.ReactElement {
  return (
    <span>
      {(ok ? a : b)}
    </span>
  );
}

import React from "react";

export interface FunctionRefProps {
  handler: any;
}

export function FunctionRef({ handler }: FunctionRefProps): React.ReactElement {
  return (
<button onClick={() => {
    handler;
}}
>
  {"click"}
</button>
  );
}
