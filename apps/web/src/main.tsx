import ReactDOM from "react-dom/client";
import { Provider } from "react-redux";
import { Toaster } from "react-hot-toast";
import { AuthProvider, AuthProviderProps } from "react-oidc-context";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Theme } from "@radix-ui/themes";
import { QueryClient, QueryClientProvider } from "react-query";
import "@radix-ui/themes/styles.css";
import "./index.css";
import Auth from "./auth/Auth.tsx";
import Home from "./pages/Home/Home.tsx";
import Task from "./pages/Task/Task.tsx";
import AuthGuard from "./auth/AuthGuard.tsx";
import Pricing from "./pages/Pricing/Pricing.tsx";
import store from "./store/store";

const oidcConfig: AuthProviderProps = {
  authority: import.meta.env.VITE_KEYCLOAK_URL,
  client_id: import.meta.env.VITE_KEYCLOAK_CLIENT_ID,
  redirect_uri: import.meta.env.VITE_KEYCLOAK_REDIRECT_URI,
  post_logout_redirect_uri: import.meta.env
    .VITE_KEYCLOAK_POST_LOGOUT_REDIRECT_URI,
  onSigninCallback: () => {
    window.history.replaceState({}, document.title, window.location.pathname);
  },
};

const router = createBrowserRouter([
  {
    path: "/",
    element: <Home />,
  },
  {
    path: "/task/:taskId/:pageCount",
    element: (
      <AuthGuard>
        <Task />
      </AuthGuard>
    ),
  },
  {
    path: "/pricing",
    element: <Pricing />,
  },
]);

const queryClient = new QueryClient();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <Theme
    scaling="100%"
    accentColor="cyan"
    panelBackground="solid"
    style={{
      height: "100%",
      backgroundColor: "hsl(192, 70%, 5%)",
    }}
  >
    <QueryClientProvider client={queryClient}>
      <AuthProvider {...oidcConfig}>
        <Provider store={store}>
          <Auth>
            <RouterProvider router={router} />
          </Auth>
        </Provider>
      </AuthProvider>
    </QueryClientProvider>
    <Toaster />
  </Theme>
);
