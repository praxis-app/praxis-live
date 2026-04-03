import { useNavigate } from "react-router-dom";

export function ErrorPage() {
  const navigate = useNavigate();

  return (
    <div className="flex min-h-screen w-full items-center justify-center pt-20">
      <img
        alt="Something went wrong"
        className="w-8/12 max-w-xl cursor-pointer rounded-full"
        onClick={() => navigate(0)}
        src="/assets/images/error.gif"
        title="Refresh"
      />
    </div>
  );
}
