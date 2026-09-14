export interface CustomCriterion {
  key: string;
  label: string;
  value_type: 'boolean' | 'text';
  value: string;
}

export interface Website {
  id: string;
  name: string;
  url: string;
  category: string;
  has_daily_checkin: boolean;
  can_cheat_account: boolean;
  requires_kyc: boolean;
  requires_proxy: boolean;
  custom_criteria: CustomCriterion[];
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
}
