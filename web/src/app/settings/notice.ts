/** Como uma parte de uma categoria avisa a categoria: um aviso de sucesso ou de erro na parte de baixo da tela. */
export type Notify = (tone: "success" | "error", text: string) => void;
