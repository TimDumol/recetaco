import { useEffect, useState, type ReactNode } from "react";
import { UserManager, WebStorageStateStore } from "oidc-client-ts";

export interface AuthConfig {
  mode: "none" | "token" | "oidc";
  oidc?: {
    issuer: string;
    client_id: string;
    android_client_id: string;
    audience: string;
  };
}
let configPromise: Promise<AuthConfig> | undefined;
let manager: UserManager | undefined;
let callback: Promise<void> | undefined;
export function authConfig(): Promise<AuthConfig> {
  return (configPromise ??= fetch("/auth/config", { cache: "no-store" })
    .then(async (r) => {
      if (!r.ok) throw new Error("Could not load sign-in configuration.");
      return r.json();
    })
    .catch((e) => {
      configPromise = undefined;
      throw e;
    }));
}
async function userManager(): Promise<UserManager> {
  if (manager) return manager;
  const { oidc } = await authConfig();
  if (!oidc) throw new Error("Pocket ID is not configured on this server.");
  manager = new UserManager({
    authority: oidc.issuer,
    client_id: oidc.client_id,
    redirect_uri: `${location.origin}/auth/callback`,
    post_logout_redirect_uri: location.origin,
    response_type: "code",
    scope: "openid profile offline_access",
    extraQueryParams: { resource: oidc.audience },
    extraTokenParams: { resource: oidc.audience },
    userStore: new WebStorageStateStore({ store: sessionStorage }),
    stateStore: new WebStorageStateStore({ store: sessionStorage }),
    automaticSilentRenew: true,
    loadUserInfo: false,
  });
  return manager;
}
export async function accessToken(): Promise<string | null> {
  const c = await authConfig();
  if (c.mode !== "oidc")
    return c.mode === "token" ? sessionStorage.getItem("recetaco-token") : null;
  const m = await userManager();
  let user = await m.getUser();
  if (user?.expired) user = await m.signinSilent();
  return user && !user.expired ? user.access_token : null;
}
export async function signIn() {
  await (await userManager()).signinRedirect();
}
export async function signOut() {
  const m = await userManager();
  // Clear the local session even if the provider's logout endpoint is unavailable.
  await m.removeUser();
  await m.signoutRedirect();
}
export function AuthGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<"loading" | "ready" | "login">(
    "loading",
  );
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);
  const [attempt, retry] = useState(0);
  useEffect(() => {
    let cancelled = false;
    let cleanup = () => {};
    async function initialize() {
      const config = await authConfig();
      if (config.mode !== "oidc") {
        if (!cancelled) setStatus("ready");
        return;
      }
      sessionStorage.removeItem("recetaco-token");
      const m = await userManager();
      if (location.pathname === "/auth/callback") {
        callback ??= m.signinRedirectCallback().then(() => {
          history.replaceState({}, "", "/");
        });
        await callback;
      }
      const user = await m.getUser();
      if (!cancelled) setStatus(user && !user.expired ? "ready" : "login");
      if (cancelled) return;
      const expired = () => {
        if (!cancelled) setStatus("login");
      };
      const loaded = () => {
        if (!cancelled) {
          setStatus("ready");
          setError("");
        }
      };
      m.events.addUserLoaded(loaded);
      m.events.addUserUnloaded(expired);
      m.events.addAccessTokenExpired(expired);
      m.events.addSilentRenewError(expired);
      cleanup = () => {
        m.events.removeUserLoaded(loaded);
        m.events.removeUserUnloaded(expired);
        m.events.removeAccessTokenExpired(expired);
        m.events.removeSilentRenewError(expired);
      };
    }
    initialize().catch(() => {
      if (!cancelled) {
        setStatus("login");
        setError(
          "Sign-in could not be completed. Try again, or check your server’s Pocket ID configuration.",
        );
      }
    });
    return () => {
      cancelled = true;
      cleanup();
    };
  }, [attempt]);
  if (status === "ready") return children;
  return (
    <div className="auth-screen">
      <div className="auth-card">
        <div className="eyebrow">
          RECETACO · YOUR EVERYDAY KITCHEN COMPANION
        </div>
        <h1>Your kitchen awaits.</h1>
        <p>
          {status === "loading"
            ? "Checking your connection…"
            : "Sign in with Pocket ID to open your recipe book, meal plan, and shopping lists."}
        </p>
        {error && (
          <div className="alert" role="alert">
            {error}
          </div>
        )}
        {status === "login" && (
          <>
            <button
              className="primary"
              disabled={pending}
              onClick={() => {
                setPending(true);
                signIn().catch(() => {
                  setError("Could not start Pocket ID sign-in.");
                  setPending(false);
                });
              }}
            >
              Sign in with Pocket ID
            </button>
            <button
              className="text-link"
              onClick={() => {
                setError("");
                setStatus("loading");
                retry((n) => n + 1);
              }}
            >
              Check connection again
            </button>
          </>
        )}
      </div>
    </div>
  );
}
