import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("tools", "routes/tools.tsx"),
  route("changelog", "routes/changelog.tsx"),
  route("*", "routes/not-found.tsx"),
] satisfies RouteConfig;
