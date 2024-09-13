import { useQuery } from "react-query";
import { getUser } from "../services/user";
import { useAuth } from "react-oidc-context";

export default function useUser() {
    const auth = useAuth();
    const { data, isLoading, error } = useQuery("user", getUser,
        {
            enabled: auth.isAuthenticated,
        }
    );
    return { data, isLoading, error };
}