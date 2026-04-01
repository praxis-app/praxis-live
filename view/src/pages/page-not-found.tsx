import { useNavigate } from "react-router-dom";

export function PageNotFound() {
  const navigate = useNavigate();

  return (
    <div className="flex min-h-screen w-full flex-col items-center justify-center">
      <img
        alt="Page not found"
        className="w-8/12 max-w-xl cursor-pointer"
        onClick={() => navigate("/")}
        src="/assets/images/404.gif"
        title="Go to home"
      />
    </div>
  );
}
