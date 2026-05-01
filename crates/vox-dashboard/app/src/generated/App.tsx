import React from "react";
import {
  Outlet,
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { AppShell } from "./AppShell.tsx";

const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

const route_0_index = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: AppShell,
});

const routeTree = rootRoute.addChildren([route_0_index]);

const router = createRouter({ routeTree });

export function App(): React.ReactElement {
  return <RouterProvider router={router} />;
}
