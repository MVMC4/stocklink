import { createContext, useContext, useState, useCallback, useMemo, type ReactNode } from 'react';
import { api, type Role, type TokenPair } from './api';

interface Session {
  accountId: string;
  roles: Role[];
}

interface AuthContextValue {
  session: Session | null;
  requestOtp: (channel: 'email' | 'phone', identifier: string) => Promise<{ devCode?: string }>;
  verifyOtp: (channel: 'email' | 'phone', identifier: string, code: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshRoles: (roles: Role[]) => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

// Only the non-secret session summary (account id + roles) lives in
// localStorage, purely so a page reload doesn't flash a logged-out state
// before the first API call resolves — it grants nothing on its own. Both
// the access and refresh tokens are HttpOnly cookies the browser sends
// automatically; this app never reads or stores either (WO-05).
const STORAGE_KEY = 'stocklink.session';

function loadSession(): Session | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Session) : null;
  } catch {
    return null;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session | null>(() => loadSession());

  const persist = useCallback((next: Session | null) => {
    setSession(next);
    if (next) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, []);

  const requestOtp = useCallback(async (channel: 'email' | 'phone', identifier: string) => {
    const res = await api.post<{ expires_in_secs: number; dev_code?: string }>('/v1/auth/otp/request', {
      channel,
      identifier,
    });
    return { devCode: res.dev_code };
  }, []);

  const verifyOtp = useCallback(
    async (channel: 'email' | 'phone', identifier: string, code: string) => {
      const res = await api.post<TokenPair>('/v1/auth/otp/verify', { channel, identifier, code });
      persist({ accountId: res.account_id, roles: res.roles });
    },
    [persist],
  );

  const logout = useCallback(async () => {
    await api.post('/v1/auth/logout', {}).catch(() => {});
    persist(null);
  }, [persist]);

  // Merges in — never replaces — because an account can hold more than one
  // role at once (see Onboarding's "add another role later"). Called right
  // after `POST /v1/onboarding/{warehouses,stores}` grants a new one; the
  // session cookie is reissued by the server in that same response, this
  // just keeps the client-held summary in sync without a round trip.
  const refreshRoles = useCallback((added: Role[]) => {
    setSession((prev) => {
      if (!prev) return prev;
      const roles = Array.from(new Set([...prev.roles, ...added]));
      const next = { ...prev, roles };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  const value = useMemo(
    () => ({ session, requestOtp, verifyOtp, logout, refreshRoles }),
    [session, requestOtp, verifyOtp, logout, refreshRoles],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
