import { useState } from 'react';
import { Mail, ArrowRight } from 'lucide-react';
import { useAuth } from '../lib/auth';
import { useToast } from '../lib/toast';
import { ApiError } from '../lib/api';

export function Login() {
  const { requestOtp, verifyOtp } = useAuth();
  const { push } = useToast();
  const [step, setStep] = useState<'identify' | 'code'>('identify');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [devCode, setDevCode] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleRequestCode(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const { devCode } = await requestOtp('email', email);
      setDevCode(devCode);
      setStep('code');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Something went wrong. Try again.');
    } finally {
      setBusy(false);
    }
  }

  async function handleVerify(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await verifyOtp('email', email, code);
      push('Signed in');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Invalid code. Try again.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-screen">
      <div className="glass auth-card">
        <div className="brand">
          <span className="brand-mark">SL</span>
          StockLink
        </div>
        {step === 'identify' ? (
          <>
            <h1>Sign in</h1>
            <p className="lead">Warehouses and stores sign in with a one-time code sent to their email.</p>
            <form onSubmit={handleRequestCode}>
              <div className="field">
                <label htmlFor="email">Work email</label>
                <input
                  id="email"
                  className="input"
                  type="email"
                  required
                  placeholder="you@business.co.bw"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
              {error && <p className="field-error">{error}</p>}
              <div className="form-actions">
                <button className="btn" type="submit" disabled={busy}>
                  {busy ? 'Sending…' : 'Send code'} <ArrowRight size={15} />
                </button>
              </div>
            </form>
          </>
        ) : (
          <>
            <h1>Enter your code</h1>
            <p className="lead">
              <Mail size={13} style={{ verticalAlign: -2 }} /> Sent to {email}
            </p>
            <form onSubmit={handleVerify}>
              <div className="field">
                <label htmlFor="code">6-digit code</label>
                <input
                  id="code"
                  className="input"
                  inputMode="numeric"
                  autoFocus
                  required
                  maxLength={6}
                  placeholder="000000"
                  value={code}
                  onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
                />
              </div>
              {error && <p className="field-error">{error}</p>}
              <div className="form-actions">
                <button className="btn" type="submit" disabled={busy || code.length !== 6}>
                  {busy ? 'Verifying…' : 'Verify'}
                </button>
                <button
                  type="button"
                  className="btn ghost"
                  onClick={() => {
                    setStep('identify');
                    setCode('');
                    setError(null);
                  }}
                >
                  Use another email
                </button>
              </div>
            </form>
            {devCode && (
              <p className="dev-code-hint">
                Development mode — your code is <strong>{devCode}</strong>
              </p>
            )}
          </>
        )}
      </div>
    </div>
  );
}
