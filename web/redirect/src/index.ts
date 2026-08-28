const redirectWorker = {
  fetch(request: Request): Response {
    const url = new URL(request.url);
    url.host = "ghosty.net";
    return Response.redirect(url.toString(), 301);
  },
};

export default redirectWorker;
