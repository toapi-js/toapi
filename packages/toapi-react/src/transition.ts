import {
  startTransition,
  type TransitionFunction,
  type TransitionStartFunction,
} from "react";

export type TransitionProp = boolean | TransitionStartFunction;

export function runTransition(tr: TransitionProp, scope: TransitionFunction) {
  switch (tr) {
    case true:
      startTransition(scope);
      return;
    case false:
      scope();
      return;
    default:
      tr(scope);
      return;
  }
}
