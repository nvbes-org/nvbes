export interface BehavioralSignals {
  mouse_event_count: number;
  mouse_variance: number;
  mouse_path_length: number;
  kb_dwell_variance: number | null;
  kb_dwell_mean: number | null;
  kb_flight_variance: number | null;
  kb_flight_mean: number | null;
  kb_sample_count: number;
  event_cascade: number;
  event_order_valid: boolean | null;
  keyboard_submit: boolean;
  email_had_focus: boolean;
  email_focus_before_value: boolean;
  caret_at_end: boolean | null;
  scroll_event_count: number;
  scroll_speed_variance: number;
  submit_visible: boolean | null;
}

export interface BehavioralObserver {
  collect: () => BehavioralSignals;
  destroy: () => void;
}
