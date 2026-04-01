import { createBrowserRouter } from "react-router-dom";
import App from "./App";
import { ErrorPage } from "./pages/error-page";
import { PageNotFound } from "./pages/page-not-found";

export const appRouter = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    errorElement: <ErrorPage />,
  },
  {
    path: "*",
    element: <PageNotFound />,
  },
]);
