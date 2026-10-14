# ADR-0011: Nome e frameworks-alvo do runtime compartilhado

- Status: Proposta
- Data: 2026-10-14

## Contexto

O PRD (§4.1) chama o runtime de sessões de `Console.Runtime`. No esqueleto, esse era também o nome do projeto e do namespace raiz. Um namespace `Console` no escopo global esconde `System.Console` em todo projeto que referencia o assembly (Core, Host, Themes, UI e testes): `Console.WriteLine(...)` passa a falhar com CS0234.

A hipótese H3 (PRD §2) exige que o Console Mode, hoje em .NET 8, consuma o mesmo runtime. Uma biblioteca com alvo apenas `net10.0` não pode ser referenciada por um app .NET 8. A migração do Console Mode para .NET 10 (PRD §3) ainda não tem data.

## Decisão

1. O projeto e o namespace passam a se chamar `ConsoleMode.Runtime`. "Console Runtime" continua sendo o nome do componente nos documentos de produto.
2. `ConsoleMode.Runtime` tem como alvos `net8.0;net10.0` e contém apenas contratos e lógica sem dependência de plataforma. Implementações Win32 (monitores, áudio, FSE) ficarão em um projeto separado com alvo `-windows`, quando forem criadas.
3. Quando o Console Mode migrar para .NET 10, o alvo `net8.0` é removido. O suporte ao .NET 8 termina em novembro de 2026.

## Consequências

- O código dos contratos não pode usar APIs exclusivas do .NET 9+ enquanto o alvo `net8.0` existir. O build de ambos os alvos no CI garante isso.
- Os testes do runtime ficam em `tests/ConsoleMode.Runtime.Tests`, separados dos testes do Core.
- O SDK 10 compila `net8.0` sem o runtime 8 instalado. Executar testes em `net8.0` exigiria instalá-lo no CI; por ora os testes rodam só em `net10.0`.
