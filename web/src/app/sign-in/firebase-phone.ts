// Firebase phone sign-in (D-12): Firebase sends and checks the SMS code; the API only ever sees the resulting ID token.
// The SDK loads only when an employee uses this form.
export type PhoneStep = { confirm: (code: string) => Promise<string> };

export async function signInWithFirebase(mobile: string, recaptchaElementId: string): Promise<PhoneStep> {
  const [{ initializeApp, getApps }, { getAuth, RecaptchaVerifier, signInWithPhoneNumber }] = await Promise.all([
    import("firebase/app"),
    import("firebase/auth"),
  ]);
  const app =
    getApps()[0] ??
    initializeApp({
      apiKey: process.env.NEXT_PUBLIC_FIREBASE_API_KEY,
      authDomain: process.env.NEXT_PUBLIC_FIREBASE_AUTH_DOMAIN,
      projectId: process.env.NEXT_PUBLIC_FIREBASE_PROJECT_ID,
    });
  const auth = getAuth(app);
  const verifier = new RecaptchaVerifier(auth, recaptchaElementId, { size: "invisible" });
  const result = await signInWithPhoneNumber(auth, mobile, verifier);
  return {
    confirm: async (code) => {
      const cred = await result.confirm(code);
      const token = await cred.user.getIdToken();
      await auth.signOut(); // our own session takes over; Firebase keeps nothing in the browser
      return token;
    },
  };
}
