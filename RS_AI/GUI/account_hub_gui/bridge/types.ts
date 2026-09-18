export interface Criterion {
  id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface LoginMethod {
  id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface Website {
  id: string;
  name: string;
  url: string;
  tags: string[];
  has_daily_checkin: boolean;
  criterion_ids: string[];
  login_method_ids: string[];
  notes: string;
  created_at: string;
}

export interface EmailAccount {
  id: string;
  email: string;
  owner: string;
  recovery_email: string;
  phone: string;
  tags: string[];
  notes: string;
  created_at: string;
}

export type AccountStatus = 'live' | 'die';

export interface RegistrationRecord {
  id: string;
  email_id: string;
  website_id: string;
  is_registered: boolean;
  status: AccountStatus; // 'live' | 'die'
  is_checked_in: boolean; // Đã điểm danh HÔM NAY chưa (tự reset khi sang ngày mới)
  registered_at?: string;
  checkin_streak: number;
  last_checkin_at?: string;
  notes: string;
}

export interface AppSettings {
  lang: string;
  theme: string;
  font: string;
}

export interface AppDatabase {
  emails: EmailAccount[];
  websites: Website[];
  registrations: RegistrationRecord[];
  settings: AppSettings;
  criteria: Criterion[];
  login_methods: LoginMethod[];
  schema_version: number;
}
