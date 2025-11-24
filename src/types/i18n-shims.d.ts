import "vue";

declare module "vue" {
  // vue-i18n references this type but it is not exported by Vue's current typings.
  export type GenericComponentInstance = any;
}
