import React from 'react';
import type { TermShellModeId } from '../../../lib/terminalBridge';
import type { ShellMode as PromptShellMode } from './intentRouter';
import {
  ScriptoriumPromptEditor,
  type ExternalDraftRequest,
  type ScriptoriumPromptEditorProps,
} from './ScriptoriumPromptEditor';

export type ShellMode = TermShellModeId;
export type { PromptShellMode, ExternalDraftRequest };
export type InputEditorProps = ScriptoriumPromptEditorProps;

export function InputEditor(props: InputEditorProps) {
  return <ScriptoriumPromptEditor {...props} />;
}
